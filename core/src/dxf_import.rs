//! Sabit ürün DXF importer'ı — plan §9, Aşama C.
//!
//! Sözleşme: FOOTPRINT / SAFETY_ZONE katmanları, `$INSUNITS = 4` (mm),
//! kapalı ve düz kenarlı 2D LWPOLYLINE / POLYLINE konturları. Seçili katmanlardaki
//! desteklenmeyen entity'ler sessizce atlanmaz. Safety zone tek kapalı poligondur.

use dxf::entities::EntityType;
use dxf::enums::Units;

use crate::error::ImportError;
use crate::geometry::{
    AREA_EPSILON_MM2, area_weighted_centroid, canonicalize, dedupe_consecutive, is_simple_polygon,
    shoelace_area, strictly_within,
};
use crate::model::{PlacementRole, ProductGeometry};

pub const FOOTPRINT_LAYER: &str = "FOOTPRINT";
pub const SAFETY_ZONE_LAYER: &str = "SAFETY_ZONE";

/// DXF byte'ını doğrulanmış, canonical (ortak origin'li) ürün geometrisine çevirir.
pub fn import_product(id: &str, bytes: &[u8]) -> Result<ProductGeometry, ImportError> {
    let drawing = dxf::Drawing::load(&mut std::io::Cursor::new(bytes))?;

    match drawing.header.default_drawing_units {
        Units::Unitless => return Err(ImportError::UnitUnspecified),
        Units::Millimeters => {}
        units => return Err(ImportError::UnitNotSupported(units)),
    }

    let mut footprints = Vec::new();
    let mut safety_zones = Vec::new();
    for entity in drawing.entities() {
        match entity.common.layer.as_str() {
            FOOTPRINT_LAYER => footprints.push(contour(&entity.specific)?),
            SAFETY_ZONE_LAYER => safety_zones.push(contour(&entity.specific)?),
            // Diğer katmanlar bilgi notuyla yok sayılır (plan §9.3).
            _ => {}
        }
    }

    if footprints.is_empty() {
        return Err(ImportError::MissingLayer(FOOTPRINT_LAYER));
    }
    match safety_zones.len() {
        0 => return Err(ImportError::MissingLayer(SAFETY_ZONE_LAYER)),
        1 => {}
        _ => {
            return Err(ImportError::InvalidPolygon(
                "SAFETY_ZONE must contain exactly one closed contour",
            ));
        }
    }

    let footprints: Vec<_> = footprints
        .into_iter()
        .map(validate_contour)
        .collect::<Result<_, _>>()?;
    let safety = validate_contour(safety_zones.remove(0))?;

    let (canonical_footprints, canonical_safety, offset) = canonicalize(&footprints, &safety);

    for footprint in &canonical_footprints {
        if !strictly_within(&canonical_safety, footprint) {
            return Err(ImportError::FootprintOutsideSafety);
        }
    }

    let footprint_area_mm2: f64 = canonical_footprints.iter().map(|p| shoelace_area(p)).sum();
    let footprint_centroid_local = area_weighted_centroid(&canonical_footprints);

    Ok(ProductGeometry {
        id: id.to_owned(),
        footprint_polygons: canonical_footprints,
        safety_area_mm2: shoelace_area(&canonical_safety),
        footprint_area_mm2,
        footprint_centroid_local,
        safety_zone: canonical_safety,
        placement_role: PlacementRole::default(),
        tags: Vec::new(),
        age_group: None,
        source_metadata: Some(format!("offset=({:.3},{:.3})", offset[0], offset[1])),
    })
}

/// Desteklenen DXF alt kümesini kapalı f64 kontura çevirir (plan §9.3).
/// POLYLINE flag maskeleri: closed=1, curve_fit=2, spline_fit=4, 3d=8, mesh=16, polyface=64.
fn contour(entity: &EntityType) -> Result<Vec<[f64; 2]>, ImportError> {
    match entity {
        EntityType::LwPolyline(poly) => {
            if poly.vertices.iter().any(|v| v.bulge != 0.0) {
                return Err(ImportError::UnsupportedEntity("LWPOLYLINE bulge"));
            }
            if !poly.is_closed() {
                return Err(ImportError::OpenContour);
            }
            Ok(poly.vertices.iter().map(|v| [v.x, v.y]).collect())
        }
        EntityType::Polyline(poly) => {
            const CLOSED: i32 = 1;
            const CURVE_FIT: i32 = 2;
            const SPLINE_FIT: i32 = 4;
            const IS_3D_POLYLINE: i32 = 8;
            const IS_3D_MESH: i32 = 16;
            const IS_POLYFACE: i32 = 64;
            if poly.flags & (IS_3D_POLYLINE | IS_3D_MESH | IS_POLYFACE) != 0 {
                return Err(ImportError::UnsupportedEntity("3D POLYLINE / mesh"));
            }
            if poly.flags & (CURVE_FIT | SPLINE_FIT) != 0 {
                return Err(ImportError::UnsupportedEntity("fitted POLYLINE"));
            }
            if poly.flags & CLOSED == 0 {
                return Err(ImportError::OpenContour);
            }
            Ok(points(poly.vertices().map(|v| &v.location)))
        }
        other => Err(ImportError::UnsupportedEntity(name_of(other))),
    }
}

fn points<'a>(locations: impl IntoIterator<Item = &'a dxf::Point>) -> Vec<[f64; 2]> {
    locations.into_iter().map(|p| [p.x, p.y]).collect()
}

/// Kontur geçerliliği: yeterli farklı köşe, sonlu koordinat, sıfıra yakın olmayan alan.
fn validate_contour(raw: Vec<[f64; 2]>) -> Result<Vec<[f64; 2]>, ImportError> {
    let polygon = dedupe_consecutive(&raw);
    let all_finite = polygon.iter().all(|p| p[0].is_finite() && p[1].is_finite());
    if !all_finite || polygon.len() < 3 {
        return Err(ImportError::InvalidPolygon(
            "contour needs 3+ distinct finite vertices",
        ));
    }
    if shoelace_area(&polygon) <= AREA_EPSILON_MM2 {
        return Err(ImportError::InvalidPolygon("contour area is near zero"));
    }
    if !is_simple_polygon(&polygon) {
        return Err(ImportError::InvalidPolygon("contour is self-intersecting"));
    }
    Ok(polygon)
}

fn name_of(entity: &EntityType) -> &'static str {
    match entity {
        EntityType::Arc(_) => "ARC",
        EntityType::Circle(_) => "CIRCLE",
        EntityType::Spline(_) => "SPLINE",
        EntityType::Insert(_) => "INSERT",
        EntityType::Line(_) => "LINE",
        _ => "entity type",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_codes_map_to_dictionary() {
        assert_eq!(ImportError::UnitUnspecified.code(), "UNIT_UNSPECIFIED");
        assert_eq!(
            ImportError::FootprintOutsideSafety.code(),
            "FOOTPRINT_OUTSIDE_SAFETY"
        );
    }
}
