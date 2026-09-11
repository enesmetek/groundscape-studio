//! Faz-1 arama davranışını MVP-1 (main) ile aynı fixture seti ve seed'ler
//! üzerinde karşılaştırmak için ölçüm örneği — MVP-2 plan P5.
//!
//! Çıktı satırları:
//!   RESULT <set> <seed> <status> <candidates> <restarts> <time_ms> <score>
//!   SCORE  <set> <seed> <anchor> <balance> <distribution> <spacing> <orientation>

use groundscape_core::{
    LayoutObjective, PlacementRole, PlacementSession, Polygon, ProductGeometry, ScoredItem,
    SearchConfig, score_components, score_layout, search_placement,
};
use std::time::Instant;

fn product(id: &str, size: f64, role: PlacementRole) -> ProductGeometry {
    let safety: Polygon = vec![[0.0, 0.0], [size, 0.0], [size, size], [0.0, size]];
    let footprint: Polygon = vec![
        [100.0, 100.0],
        [size - 100.0, 100.0],
        [size - 100.0, size - 100.0],
        [100.0, size - 100.0],
    ];
    ProductGeometry {
        id: id.to_owned(),
        footprint_polygons: vec![footprint],
        safety_zone: safety,
        safety_area_mm2: size * size,
        footprint_area_mm2: (size - 200.0) * (size - 200.0),
        footprint_centroid_local: [0.0, 0.0],
        placement_role: role,
        tags: Vec::new(),
        age_group: None,
        source_metadata: None,
    }
}

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
    let config = SearchConfig::default();
    let objective = LayoutObjective::default();
    for (name, products) in sets() {
        let mut areas: Vec<f64> = products
            .iter()
            .map(|p| p.footprint_area_mm2.abs())
            .collect();
        areas.sort_by(f64::total_cmp);
        areas.pop();
        let second_largest = areas.pop().unwrap_or(0.0);
        for seed in 1..=20u64 {
            let mut session = PlacementSession::new(&products).unwrap();
            let start = Instant::now();
            let result = search_placement(&mut session, &products, &config, &objective, seed);
            let ms = start.elapsed().as_secs_f64() * 1000.0;
            let scored: Vec<ScoredItem> = result
                .placements
                .iter()
                .filter_map(|placement| {
                    let product = products.iter().find(|p| p.id == placement.product_id)?;
                    Some(ScoredItem::new(
                        placement.pose,
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
                "RESULT {} {} {:?} {} {} {:.1} {:.4}",
                name,
                seed,
                result.status,
                result.stats.candidates_tried,
                result.stats.restarts,
                ms,
                score
            );
            println!(
                "SCORE {} {} {:.4} {:.4} {:.4} {:.4} {:.4}",
                name,
                seed,
                components[0],
                components[1],
                components[2],
                components[3],
                components[4]
            );
        }
    }
}
