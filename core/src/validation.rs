//! Orijinal canonical f64 geometri üzerinde bağımsız son doğrulama — plan §12.
//!
//! Motorun surrogate geometrisi değil, uygulamanın kendisine dönen pozlar ve
//! canonical poligonlar esas alınır. Boolean işlemlerinden önce poligon
//! geçerliliği denetlenir.

use serde::Serialize;

use crate::area::AREA_SIZE_MM;
use crate::geometry::{
    AREA_EPSILON_MM2, LINEAR_EPSILON_MM, Polygon, shoelace_area, strictly_within, transformed_bbox,
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
        if !is_simple(&product.safety_zone)
            || product.footprint_polygons.iter().any(|p| !is_simple(p))
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

/// Basit poligon: bitişik olmayan kenarlar kesişmez (O(n²); köşe sayısı küçük).
fn is_simple(polygon: &Polygon) -> bool {
    let n = polygon.len();
    if n < 3 {
        return false;
    }
    let segment = |i: usize| (polygon[i], polygon[(i + 1) % n]);
    for a in 0..n {
        let (p1, p2) = segment(a);
        for b in (a + 2)..n {
            if a == 0 && b == n - 1 {
                continue; // bitişik kenarlar (döngü kapanışı)
            }
            let (q1, q2) = segment(b);
            if segments_intersect(p1, p2, q1, q2) {
                return false;
            }
        }
    }
    true
}

fn segments_intersect(p1: [f64; 2], p2: [f64; 2], q1: [f64; 2], q2: [f64; 2]) -> bool {
    let d1 = cross(q1, q2, p1);
    let d2 = cross(q1, q2, p2);
    let d3 = cross(p1, p2, q1);
    let d4 = cross(p1, p2, q2);
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }
    // Sıfıra yakın taşma/hata payları: dokunma kabul edilir (plan §2.2 temas politikası).
    d1.abs() <= LINEAR_EPSILON_MM && on_segment(q1, q2, p1)
        || d2.abs() <= LINEAR_EPSILON_MM && on_segment(q1, q2, p2)
        || d3.abs() <= LINEAR_EPSILON_MM && on_segment(p1, p2, q1)
        || d4.abs() <= LINEAR_EPSILON_MM && on_segment(p1, p2, q2)
}

fn cross(o: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
}

fn on_segment(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> bool {
    p[0] >= a[0].min(b[0]) - LINEAR_EPSILON_MM
        && p[0] <= a[0].max(b[0]) + LINEAR_EPSILON_MM
        && p[1] >= a[1].min(b[1]) - LINEAR_EPSILON_MM
        && p[1] <= a[1].max(b[1]) + LINEAR_EPSILON_MM
}
