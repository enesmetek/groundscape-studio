//! Orijinal canonical f64 geometri üzerinde bağımsız son doğrulama — plan §12.
//!
//! Motorun surrogate geometrisi değil, uygulamanın kendisine dönen pozlar ve
//! canonical poligonlar esas alınır. Boolean işlemlerinden önce poligon
//! geçerliliği denetlenir.

use serde::Serialize;

use crate::area::AREA_SIZE_MM;
use crate::geometry::{
    AREA_EPSILON_MM2, LINEAR_EPSILON_MM, Polygon, is_simple_polygon, shoelace_area,
    strictly_within, transformed_bbox,
};
use crate::model::{Placement, ProductGeometry};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationReport {
    pub valid: bool,
    /// Plan §12 ölçüt etiketleri: ZONE_OVERLAP, OUT_OF_AREA,
    /// FOOTPRINT_OUTSIDE_SAFETY, SCALE_CHANGED, INVALID_POLYGON,
    /// DUPLICATE_PRODUCT, UNKNOWN_PRODUCT, NONFINITE_POSE.
    pub issues: Vec<&'static str>,
}

/// Verilen pozların orijinal geometri üzerindeki bağımsız denetimi.
/// Tamlık (bütün ürünler tam birer kez) arayanın sorumluluğundadır;
/// burada yalnızca verilen poz kümesi denetlenir.
#[must_use]
pub fn validate_result(products: &[ProductGeometry], placements: &[Placement]) -> ValidationReport {
    let mut issues: Vec<&'static str> = Vec::new();

    let mut by_id = std::collections::HashMap::new();
    for product in products {
        by_id.insert(product.id.as_str(), product);
    }
    let mut seen = std::collections::HashSet::new();
    for placement in placements {
        if !seen.insert(placement.product_id.as_str()) {
            issues.push("DUPLICATE_PRODUCT");
        }
        let Some(product) = by_id.get(placement.product_id.as_str()) else {
            issues.push("UNKNOWN_PRODUCT");
            continue;
        };
        let pose = placement.pose;
        if !pose.x_mm.is_finite() || !pose.y_mm.is_finite() || !pose.rotation_rad.is_finite() {
            issues.push("NONFINITE_POSE");
            continue;
        }

        // Poligon geçerliliği önce; geçersiz geometri üzerinde Boolean'a güvenilmez.
        if !is_simple_polygon(&product.safety_zone)
            || product
                .footprint_polygons
                .iter()
                .any(|p| !is_simple_polygon(p))
        {
            issues.push("INVALID_POLYGON");
            continue;
        }

        // Alan içinde kalma: footprint ve safety zone, kayıtlı sayısal politika içinde.
        let mut all_within = within_area(&product.safety_zone, &pose);
        for footprint in &product.footprint_polygons {
            all_within &= within_area(footprint, &pose);
        }
        if !all_within {
            issues.push("OUT_OF_AREA");
        }

        // Katman ilişkisi: footprint, ilgili safety zone içinde.
        for footprint in &product.footprint_polygons {
            if !strictly_within(&product.safety_zone, footprint) {
                issues.push("FOOTPRINT_OUTSIDE_SAFETY");
            }
        }

        // Ölçek: ithalattaki mm boyutları değişmemiş (alan tutarlılığı).
        if (shoelace_area(&product.safety_zone) - product.safety_area_mm2.abs()) > AREA_EPSILON_MM2
        {
            issues.push("SCALE_CHANGED");
        }
    }

    // Zone çakışması: her safety çifti için iç bölge kesişimi yok.
    for i in 0..placements.len() {
        for j in (i + 1)..placements.len() {
            if let (Some(a), Some(b)) = (
                by_id.get(placements[i].product_id.as_str()),
                by_id.get(placements[j].product_id.as_str()),
            ) && zone_overlap(
                &a.safety_zone,
                placements[i].pose,
                &b.safety_zone,
                placements[j].pose,
            ) {
                issues.push("ZONE_OVERLAP");
            }
        }
    }

    issues.dedup();
    ValidationReport {
        valid: issues.is_empty(),
        issues,
    }
}

/// İç bölge örtüşmesi: kesişim alanı `AREA_EPSILON_MM2`'den büyükse çakışma.
/// Yalnızca kenar/köşe teması (kesişim alanı ≈ 0) kabul edilir (plan §2.2).
fn zone_overlap(
    a: &Polygon,
    pose_a: crate::model::Pose,
    b: &Polygon,
    pose_b: crate::model::Pose,
) -> bool {
    use geo::algorithm::Area;
    use geo::algorithm::bool_ops::BooleanOps;
    use geo::{LineString, Polygon as GeoPolygon};

    let to_geo = |polygon: &Polygon, pose: crate::model::Pose| {
        let (sin, cos) = pose.rotation_rad.sin_cos();
        GeoPolygon::new(
            LineString::from(
                polygon
                    .iter()
                    .map(|&[x, y]| geo::Coord {
                        x: x * cos - y * sin + pose.x_mm,
                        y: x * sin + y * cos + pose.y_mm,
                    })
                    .collect::<Vec<_>>(),
            ),
            vec![],
        )
    };
    to_geo(a, pose_a)
        .intersection(&to_geo(b, pose_b))
        .unsigned_area()
        > AREA_EPSILON_MM2
}

/// Pozun poligonu alan sınırları içinde mi (LINEAR_EPSILON_MM toleranslı).
fn within_area(polygon: &Polygon, pose: &crate::model::Pose) -> bool {
    let bbox = transformed_bbox(polygon, pose.rotation_rad, [pose.x_mm, pose.y_mm]);
    bbox[0] >= -LINEAR_EPSILON_MM
        && bbox[1] >= -LINEAR_EPSILON_MM
        && bbox[2] <= AREA_SIZE_MM + LINEAR_EPSILON_MM
        && bbox[3] <= AREA_SIZE_MM + LINEAR_EPSILON_MM
}
