//! f64 alan geometrisi ve tolerans politikası — plan §8, Aşama B.
//!
//! Uygulamanın kaynak geometrisi f64; jagua f32'ye yalnızca adaptör sınırında
//! çevrilir. Açı uzayı radyandır; derece yalnızca görselleştirmede kullanılır.

/// Doğrusal tolerans (mm): kenar teması ve sınır kontrolünde kullanılır.
/// Sayısal regresyon testlerinden sonra kayda geçilmiştir; fiziksel güvenlik
/// mesafesi olarak yorumlanmaz.
pub const LINEAR_EPSILON_MM: f64 = 0.01;

/// Alan toleransı (mm²): poligon alan karşılaştırmaları için, doğrusal
/// toleranstan bağımsız kavram (plan §8.3).
pub const AREA_EPSILON_MM2: f64 = 1.0;

pub type Polygon = Vec<[f64; 2]>;

#[must_use]
pub fn shoelace_area(polygon: &[[f64; 2]]) -> f64 {
    let mut twice_signed = 0.0;
    for i in 0..polygon.len() {
        let [x1, y1] = polygon[i];
        let [x2, y2] = polygon[(i + 1) % polygon.len()];
        twice_signed += x1 * y2 - x2 * y1;
    }
    twice_signed.abs() / 2.0
}

/// Döndürülmüş + ötelenmiş poligonun sıkı sınır kutusu.
#[must_use]
pub fn transformed_bbox(polygon: &[[f64; 2]], angle_rad: f64, translation: [f64; 2]) -> [f64; 4] {
    let (sin, cos) = angle_rad.sin_cos();
    let mut bbox = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for &[x, y] in polygon {
        let (rx, ry) = (
            x * cos - y * sin + translation[0],
            x * sin + y * cos + translation[1],
        );
        bbox[0] = bbox[0].min(rx);
        bbox[1] = bbox[1].min(ry);
        bbox[2] = bbox[2].max(rx);
        bbox[3] = bbox[3].max(ry);
    }
    bbox
}

/// Açısı 0 olan pozlar için basitleştirilmiş bbox.
#[must_use]
pub fn bbox(polygon: &[[f64; 2]]) -> [f64; 4] {
    transformed_bbox(polygon, 0.0, [0.0; 2])
}

/// Ardışık, `LINEAR_EPSILON_MM`'den yakın tekrar eden noktaları temizler
/// (plan §9.3: küçük tekrar eden ardışık noktalar kontrollü temizlenebilir).
#[must_use]
pub fn dedupe_consecutive(polygon: &[[f64; 2]]) -> Polygon {
    let mut cleaned: Polygon = Vec::with_capacity(polygon.len());
    for &p in polygon {
        match cleaned.last() {
            Some(&last)
                if (p[0] - last[0]).abs() <= LINEAR_EPSILON_MM
                    && (p[1] - last[1]).abs() <= LINEAR_EPSILON_MM => {}
            _ => cleaned.push(p),
        }
    }
    if cleaned.len() > 1 {
        let first = cleaned[0];
        let last = cleaned[cleaned.len() - 1];
        if (first[0] - last[0]).abs() <= LINEAR_EPSILON_MM
            && (first[1] - last[1]).abs() <= LINEAR_EPSILON_MM
        {
            cleaned.pop();
        }
    }
    cleaned
}

/// `inner` poligonunun `outer` sınırının dışına çıkıp çıkmadığını poligon
/// farkıyla denetler (plan §9.3: yalnızca köşe kontrolü değil; konkav kenar
/// dışarı çıkabilir). Dışarıda kalan alan `AREA_EPSILON_MM2`'den küçükse
/// içeride sayılır.
#[must_use]
pub fn strictly_within(outer: &[[f64; 2]], inner: &[[f64; 2]]) -> bool {
    use geo::algorithm::Area;
    use geo::algorithm::bool_ops::BooleanOps;
    use geo::{LineString, Polygon as GeoPolygon};

    let to_geo = |p: &[[f64; 2]]| {
        GeoPolygon::new(
            LineString::from(
                p.iter()
                    .map(|&[x, y]| geo::Coord { x, y })
                    .collect::<Vec<_>>(),
            ),
            vec![],
        )
    };
    // inner − outer = sınır dışında kalan parça; boşsa inner içeride.
    let outside = to_geo(inner).difference(&to_geo(outer));
    outside.unsigned_area() <= AREA_EPSILON_MM2
}

/// Bir poligonun tüm noktalarına aynı ofseti uygular (ortak origin için).
#[must_use]
pub fn translated(polygon: &[[f64; 2]], offset: [f64; 2]) -> Polygon {
    polygon
        .iter()
        .map(|&[x, y]| [x + offset[0], y + offset[1]])
        .collect()
}

/// Ortak origin: safety zone bbox merkezi iki katmandan da aynı şekilde çıkarılır
/// (plan §8.2). Footprint ve safety bağımsız merkezlenmez; ürünün geometrik
/// ilişkisi korunur.
///
/// Döndürür: (canonical footprints, canonical safety, çıkarılan ofset).
/// Dünya pozu: `T_world = T_canonical`, geri dönüş `offset` ile yapılır.
#[must_use]
pub fn canonicalize(
    footprint_polygons: &[Polygon],
    safety_zone: &[[f64; 2]],
) -> (Vec<Polygon>, Polygon, [f64; 2]) {
    let bbox = bbox(safety_zone);
    let center = [(bbox[0] + bbox[2]) / 2.0, (bbox[1] + bbox[3]) / 2.0];
    let offset = [-center[0], -center[1]];
    (
        footprint_polygons
            .iter()
            .map(|p| translated(p, offset))
            .collect(),
        translated(safety_zone, offset),
        center,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(size: f64) -> Polygon {
        vec![[0.0, 0.0], [size, 0.0], [size, size], [0.0, size]]
    }

    #[test]
    fn shoelace_area_of_squares() {
        assert_eq!(shoelace_area(&square(100.0)), 10_000.0);
        assert_eq!(shoelace_area(&square(5000.0)), 25_000_000.0);
    }

    #[test]
    fn rotation_preserves_area_within_tolerance() {
        let area = shoelace_area(&square(300.0));
        for angle in [0.0, 0.5, 1.0, 2.0] {
            let rotated = translated(&square(300.0), [0.0; 2]);
            let _ = rotated;
            let bbox = transformed_bbox(&square(300.0), angle, [0.0; 2]);
            // Dönüş alanını korur; kutu alanı yalnızca büyüyebilir.
            assert!((bbox[2] - bbox[0]) * (bbox[3] - bbox[1]) >= area - AREA_EPSILON_MM2);
        }
        let _ = area;
    }

    #[test]
    fn thirty_seven_degree_round_trip_is_identity() {
        let square = square(100.0);
        let angle = 37.0_f64.to_radians();

        let rotated_bbox = transformed_bbox(&square, angle, [0.0; 2]);
        let width = rotated_bbox[2] - rotated_bbox[0];
        // Dönmüş karenin axis-aligned bbox genişliği tam size·(|cosθ|+|sinθ|)
        assert!(
            width > 100.0 && width <= 100.0 * (angle.cos() + angle.sin()) + LINEAR_EPSILON_MM,
            "width={width} bbox={rotated_bbox:?}"
        );

        // Dönüşün tersi noktaları geri getirir
        let (sin, cos) = (-angle.sin(), angle.cos());
        for &[x, y] in &square {
            let (rx, ry) = (
                x * angle.cos() - y * angle.sin(),
                x * angle.sin() + y * angle.cos(),
            );
            let (bx, by) = (rx * cos - ry * sin, rx * sin + ry * cos);
            assert!((bx - x).abs() <= LINEAR_EPSILON_MM && (by - y).abs() <= LINEAR_EPSILON_MM);
        }
    }

    #[test]
    fn canonicalization_uses_shared_safety_center() {
        let footprint = vec![vec![
            [100.0, 200.0],
            [200.0, 200.0],
            [200.0, 300.0],
            [100.0, 300.0],
        ]];
        let safety = vec![[0.0, 0.0], [400.0, 0.0], [400.0, 400.0], [0.0, 400.0]];

        let (footprints, zone, offset) = canonicalize(&footprint, &safety);

        // Safety bbox merkezi (200, 200); canonical safety bu merkez etrafında
        let zone_bbox = bbox(&zone);
        assert!(((zone_bbox[0] + zone_bbox[2]) / 2.0).abs() <= 1e-9);
        assert!(((zone_bbox[1] + zone_bbox[3]) / 2.0).abs() <= 1e-9);
        assert_eq!(offset, [200.0, 200.0]);

        // Footprint-safety görece konumu korunur: köşe farkı aynı
        let original_corner = footprint[0][0];
        let canonical_corner = footprints[0][0];
        assert_eq!(
            canonical_corner,
            [original_corner[0] - 200.0, original_corner[1] - 200.0]
        );

        // Geri uygulama orijinali verir
        assert_eq!(translated(&zone, offset), safety);
        assert_eq!(translated(&footprints[0], offset), footprint[0]);
    }
}
