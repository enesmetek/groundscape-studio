use groundscape_core::{
    PlacementSession, Polygon, ProductGeometry, SearchConfig, Status, search_placement,
};

fn product(id: &str, size: f64) -> ProductGeometry {
    let safety: Polygon = vec![[0.0, 0.0], [size, 0.0], [size, size], [0.0, size]];
    let footprint: Polygon = vec![
        [50.0, 50.0],
        [size - 50.0, 50.0],
        [size - 50.0, size - 50.0],
        [50.0, size - 50.0],
    ];
    ProductGeometry {
        id: id.to_owned(),
        footprint_polygons: vec![footprint],
        safety_zone: safety,
        safety_area_mm2: size * size,
        source_metadata: None,
    }
}

fn fast_config() -> SearchConfig {
    SearchConfig {
        global_samples_per_item: 400,
        local_samples_per_item: 100,
        candidate_buffer_size: 6,
        max_restarts: 1,
        max_total_candidates: 20_000,
    }
}

fn rectangle_product(id: &str, width: f64, height: f64) -> ProductGeometry {
    ProductGeometry {
        id: id.to_owned(),
        footprint_polygons: vec![vec![
            [10.0, 10.0],
            [width - 10.0, 10.0],
            [width - 10.0, height - 10.0],
            [10.0, height - 10.0],
        ]],
        safety_zone: vec![[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]],
        safety_area_mm2: width * height,
        source_metadata: None,
    }
}

#[test]
fn three_products_search_completes_in_order() {
    let products = [
        product("big", 1200.0),
        product("mid", 900.0),
        product("small", 700.0),
    ];
    let mut session = PlacementSession::new(&products).unwrap();

    let result = search_placement(&mut session, &products, &fast_config(), 42);
    assert_eq!(result.status, Status::Complete, "{result:?}");
    assert_eq!(result.unplaced.len(), 0);
    // Sıra: alan azalan — ilk yerleşim en büyük ürün
    assert_eq!(result.placements[0].product_id, "big");
    assert_eq!(result.placements[1].product_id, "mid");
    assert_eq!(result.placements[2].product_id, "small");
    assert_eq!(result.reason_code, "");
}

#[test]
fn total_area_exceeded_is_provable_rejection() {
    let products = [product("a", 4000.0), product("b", 4000.0)]; // 32e6 > 25e6
    let mut session = PlacementSession::new(&products).unwrap();

    let result = search_placement(&mut session, &products, &fast_config(), 42);
    assert_eq!(result.status, Status::InvalidInput);
    assert_eq!(result.reason_code, "NECESSARY_AREA_EXCEEDED");
    assert_eq!(result.stats.candidates_tried, 0);
    assert_eq!(result.unplaced.len(), 2);
}

#[test]
fn geometrically_impossible_hits_budget_not_claimed_impossible() {
    // 3500² x2: toplam 24.5e6 ≤ 25e6 ama 5000² alana birlikte sığmaz
    let products = [product("a", 3500.0), product("b", 3500.0)];
    let mut session = PlacementSession::new(&products).unwrap();

    let result = search_placement(&mut session, &products, &fast_config(), 7);
    assert_ne!(result.status, Status::Complete);
    assert_eq!(result.reason_code, "SEARCH_BUDGET_EXHAUSTED");
    // Bütçe saygılı: erken elenen denemeler de sayılır
    assert!(result.stats.candidates_tried > 0);
    assert!(result.stats.candidates_tried <= 20_000);
    assert_eq!(result.placements.len(), 1); // büyükten küçüğe: ilki yerleşir
    assert_eq!(result.placements[0].product_id, "a");
}

#[test]
fn same_seed_is_deterministic() {
    let products = [
        product("big", 1200.0),
        product("mid", 900.0),
        product("small", 700.0),
    ];
    let mut session = PlacementSession::new(&products).unwrap();
    let first = search_placement(&mut session, &products, &fast_config(), 99);
    let mut session = PlacementSession::new(&products).unwrap();
    let second = search_placement(&mut session, &products, &fast_config(), 99);
    assert_eq!(first.placements, second.placements);
    assert_eq!(first.stats, second.stats);
}

#[test]
fn rotated_only_fit_is_found_by_continuous_angles() {
    // 5500×500 dikdörtgen eksen hizalı sığmaz; ~33°-40° penceresinde sığar.
    // Dört sabit açıyla çalışan sahte "serbest rotasyon"u yakalar (plan §15).
    let products = [rectangle_product("bar", 5500.0, 500.0)];
    let mut session = PlacementSession::new(&products).unwrap();

    let result = search_placement(&mut session, &products, &fast_config(), 3);
    assert_eq!(result.status, Status::Complete, "{result:?}");
    let pose = result.placements[0].pose;
    let deg = pose.rotation_rad.to_degrees();
    assert!(
        (30.0..45.0).contains(&deg) || (210.0..225.0).contains(&deg),
        "unexpected angle {deg}"
    );
}
