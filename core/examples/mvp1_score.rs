//! MVP-1 yerleşimlerini Faz-1 skor fonksiyonuyla puanlar (MVP-2 plan P5).
//! Girdi: `mvp1_bench` örneğinin çıktısı (RESULT/PLACE satırları).
//!
//! Kullanım: cargo run --release -p groundscape-core --example mvp1_score -- <mvp1_bench.txt|->

use groundscape_core::{
    LayoutObjective, PlacementRole, Polygon, Pose, ProductGeometry, ScoredItem,
    area_weighted_centroid, score_components, score_layout,
};
use std::{fs, io::Read};

fn product(id: &str, size: f64, role: PlacementRole) -> ProductGeometry {
    let half = size / 2.0;
    let safety: Polygon = vec![[-half, -half], [half, -half], [half, half], [-half, half]];
    let footprint: Polygon = vec![
        [100.0 - half, 50.0 - half],
        [half - 150.0, 50.0 - half],
        [half - 150.0, half - 100.0],
        [100.0 - half, half - 50.0],
    ];
    let centroid = area_weighted_centroid(std::slice::from_ref(&footprint));
    ProductGeometry {
        id: id.to_owned(),
        footprint_polygons: vec![footprint],
        safety_zone: safety,
        safety_area_mm2: size * size,
        footprint_area_mm2: (size - 250.0) * (size - 125.0),
        footprint_centroid_local: centroid,
        placement_role: role,
        tags: Vec::new(),
        age_group: None,
        source_metadata: None,
    }
}

/// Ölçüm setleri; `faz1_bench` ile aynı.
fn sets() -> Vec<(&'static str, Vec<ProductGeometry>)> {
    vec![
        (
            "buyuk1kucuk6",
            vec![product("big", 1200.0, PlacementRole::Auto)]
                .into_iter()
                .chain((0..6).map(|i| product(&format!("s{i}"), 500.0, PlacementRole::Auto)))
                .collect(),
        ),
        (
            "fitness7",
            (0..7)
                .map(|i| product(&format!("f{i}"), 600.0, PlacementRole::Auto))
                .collect(),
        ),
        (
            "peripheral",
            vec![
                product("peripheral", 1400.0, PlacementRole::Peripheral),
                product("big", 1200.0, PlacementRole::Auto),
            ]
            .into_iter()
            .chain((0..4).map(|i| product(&format!("s{i}"), 500.0, PlacementRole::Auto)))
            .collect(),
        ),
        (
            "ikibuyuk",
            vec![
                product("big1", 1500.0, PlacementRole::Auto),
                product("big2", 1450.0, PlacementRole::Auto),
            ]
            .into_iter()
            .chain((0..4).map(|i| product(&format!("s{i}"), 500.0, PlacementRole::Auto)))
            .collect(),
        ),
    ]
}

fn main() {
    let path = std::env::args().nth(1).expect("mvp1 output path required");
    let objective = LayoutObjective::default();
    let text = if path == "-" {
        let mut text = String::new();
        std::io::stdin()
            .read_to_string(&mut text)
            .expect("read mvp1 output from stdin");
        text
    } else {
        fs::read_to_string(&path).expect("read mvp1 output")
    };

    // PLACE satırlarını (set, seed) grubuna topla.
    let mut groups: std::collections::BTreeMap<(String, u64), Vec<(String, Pose)>> =
        std::collections::BTreeMap::new();
    for line in text.lines().filter(|l| l.starts_with("PLACE ")) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        // PLACE <set> <seed> <id> <x> <y> <deg>
        let key = (parts[1].to_owned(), parts[2].parse::<u64>().expect("seed"));
        let pose = Pose::new(
            parts[4].parse().expect("x"),
            parts[5].parse().expect("y"),
            parts[6].parse::<f64>().expect("deg").to_radians(),
        );
        groups
            .entry(key.clone())
            .or_default()
            .push((parts[3].to_owned(), pose));
    }

    for ((set, seed), placements) in &groups {
        let products = sets()
            .into_iter()
            .find(|(n, _)| n == set)
            .map(|(_, p)| p)
            .expect("unknown set");
        let mut areas: Vec<f64> = products
            .iter()
            .map(|p| p.footprint_area_mm2.abs())
            .collect();
        areas.sort_by(f64::total_cmp);
        areas.pop();
        let second_largest = areas.pop().unwrap_or(0.0);
        let scored: Vec<ScoredItem> = placements
            .iter()
            .filter_map(|(id, pose)| {
                let product = products.iter().find(|p| p.id == *id)?;
                Some(ScoredItem::new(
                    *pose,
                    product.footprint_centroid_local,
                    product.footprint_area_mm2,
                    product.placement_role,
                    second_largest,
                ))
            })
            .collect();
        let components = score_components(&scored, &objective);
        let score = score_layout(&scored, &objective);
        println!(
            "SCORE {} {} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            set,
            seed,
            components[0],
            components[1],
            components[2],
            components[3],
            components[4],
            score
        );
    }
}
