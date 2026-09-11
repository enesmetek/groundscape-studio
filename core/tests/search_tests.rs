use groundscape_core::{
    LayoutObjective, PlacementRole, PlacementSession, Polygon, ProductGeometry, SearchConfig,
    Status, area_weighted_centroid, search_placement,
};

fn product(id: &str, size: f64) -> ProductGeometry {
    let half = size / 2.0;
    let safety: Polygon = vec![[-half, -half], [half, -half], [half, half], [-half, half]];
    // Asimetrik canonical footprint: footprint_centroid_local ≠ (0,0) ve
    // skorlamadaki dünya merkezi rotasyona duyarlıdır (plan §4.3, P5.5).
    let footprint: Polygon = vec![
        [100.0 - half, 50.0 - half],
        [half - 150.0, 50.0 - half],
        [half - 150.0, half - 100.0],
        [100.0 - half, half - 50.0],
    ];
    // Centroid'i sabit yazma; geometriden hesapla (dxf_import ile aynı yol).
    // Canonical safety merkezi origin'dir.
    let centroid = area_weighted_centroid(std::slice::from_ref(&footprint));
    ProductGeometry {
        id: id.to_owned(),
        footprint_polygons: vec![footprint],
        safety_zone: safety,
        safety_area_mm2: size * size,
        footprint_area_mm2: (size - 250.0) * (size - 125.0),
        footprint_centroid_local: centroid,
        placement_role: PlacementRole::Auto,
        tags: Vec::new(),
        age_group: None,
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
    let (half_width, half_height) = (width / 2.0, height / 2.0);
    ProductGeometry {
        id: id.to_owned(),
        footprint_polygons: vec![vec![
            [10.0 - half_width, 10.0 - half_height],
            [half_width - 10.0, 10.0 - half_height],
            [half_width - 10.0, half_height - 10.0],
            [10.0 - half_width, half_height - 10.0],
        ]],
        safety_zone: vec![
            [-half_width, -half_height],
            [half_width, -half_height],
            [half_width, half_height],
            [-half_width, half_height],
        ],
        safety_area_mm2: width * height,
        footprint_area_mm2: (width - 20.0) * (height - 20.0),
        footprint_centroid_local: [0.0, 0.0],
        placement_role: PlacementRole::Auto,
        tags: Vec::new(),
        age_group: None,
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

    let result = search_placement(
        &mut session,
        &products,
        &fast_config(),
        &LayoutObjective::default(),
        42,
    );
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

    let result = search_placement(
        &mut session,
        &products,
        &fast_config(),
        &LayoutObjective::default(),
        42,
    );
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

    let result = search_placement(
        &mut session,
        &products,
        &fast_config(),
        &LayoutObjective::default(),
        7,
    );
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
    let first = search_placement(
        &mut session,
        &products,
        &fast_config(),
        &LayoutObjective::default(),
        99,
    );
    let mut session = PlacementSession::new(&products).unwrap();
    let second = search_placement(
        &mut session,
        &products,
        &fast_config(),
        &LayoutObjective::default(),
        99,
    );
    assert_eq!(first.placements, second.placements);
    assert_eq!(first.stats, second.stats);
}

#[test]
fn rotated_only_fit_is_found_by_continuous_angles() {
    // 5500×500 dikdörtgen eksen hizalı sığmaz; ~33°-40° penceresinde sığar.
    // Dört sabit açıyla çalışan sahte "serbest rotasyon"u yakalar (plan §15).
    let products = [rectangle_product("bar", 5500.0, 500.0)];
    let mut session = PlacementSession::new(&products).unwrap();

    let result = search_placement(
        &mut session,
        &products,
        &fast_config(),
        &LayoutObjective::default(),
        3,
    );
    assert_eq!(result.status, Status::Complete, "{result:?}");
    let pose = result.placements[0].pose;
    let deg = pose.rotation_rad.to_degrees();
    // Dikdörtgen 180°-simetrik; dört eşdeğer pencere de geçerli çözümdür.
    let windows = [(30.0, 45.0), (135.0, 150.0), (210.0, 225.0), (315.0, 330.0)];
    assert!(
        windows.iter().any(|&(lo, hi)| deg >= lo && deg <= hi),
        "unexpected angle {deg}"
    );
}

/// Kabul senaryosu (MVP-2 plan §7, Faz 1): büyük safety'li `peripheral` ürün
/// merkezi bölgeye alınmaz.
#[test]
fn peripheral_product_avoids_center_region() {
    let mut peripheral = product("peripheral", 1400.0);
    peripheral.placement_role = PlacementRole::Peripheral;
    let products = [peripheral, product("big", 1200.0)];
    let mut session = PlacementSession::new(&products).unwrap();

    let result = search_placement(
        &mut session,
        &products,
        &fast_config(),
        &LayoutObjective::default(),
        5,
    );
    assert_eq!(result.status, Status::Complete, "{result:?}");
    let pose = &result
        .placements
        .iter()
        .find(|p| p.product_id == "peripheral")
        .unwrap()
        .pose;
    let half = 0.25 * 5000.0;
    let outside = (pose.x_mm - 2500.0).abs() > half || (pose.y_mm - 2500.0).abs() > half;
    assert!(outside, "peripheral pose centered: {pose:?}");
}

/// Kabul senaryosu (MVP-2 plan §7, Faz 1): bir büyük + altı küçük — büyük
/// ürünün footprint merkezi merkezi bölgede kalır.
#[test]
fn big_product_footprint_center_stays_in_center_region() {
    let products = [
        product("big", 1200.0),
        product("s1", 500.0),
        product("s2", 500.0),
        product("s3", 500.0),
        product("s4", 500.0),
        product("s5", 500.0),
        product("s6", 500.0),
    ];
    let mut session = PlacementSession::new(&products).unwrap();

    let result = search_placement(
        &mut session,
        &products,
        &fast_config(),
        &LayoutObjective::default(),
        21,
    );
    assert_eq!(result.status, Status::Complete, "{result:?}");
    let pose = &result
        .placements
        .iter()
        .find(|p| p.product_id == "big")
        .unwrap()
        .pose;
    let center =
        groundscape_core::world_footprint_center(*pose, products[0].footprint_centroid_local);
    let half = 0.25 * 5000.0;
    // Asimetrik footprint merkezi pozu en fazla ~28 mm kaydırır — tolerans buna göre.
    let margin = half + 30.0;
    assert!(
        (center[0] - 2500.0).abs() <= margin && (center[1] - 2500.0).abs() <= margin,
        "big product center {center:?} outside region"
    );
}

#[test]
fn invalid_objective_is_rejected_before_search() {
    let products = [product("a", 1000.0)];
    let mut session = PlacementSession::new(&products).unwrap();
    let objective = LayoutObjective {
        grid: 4,
        ..LayoutObjective::default()
    };

    let result = search_placement(&mut session, &products, &fast_config(), &objective, 1);
    assert_eq!(result.status, Status::InvalidInput);
    assert_eq!(result.reason_code, "INVALID_LAYOUT_OBJECTIVE");
    assert_eq!(result.stats.candidates_tried, 0);
}

#[test]
fn exhausted_sampling_is_not_reported_as_budget_exhaustion() {
    let products = [product("a", 1000.0)];
    let mut session = PlacementSession::new(&products).unwrap();
    let config = SearchConfig {
        global_samples_per_item: 0,
        local_samples_per_item: 0,
        candidate_buffer_size: 6,
        max_restarts: 0,
        max_total_candidates: 100,
    };

    let result = search_placement(
        &mut session,
        &products,
        &config,
        &LayoutObjective::default(),
        1,
    );
    assert_eq!(result.status, Status::NoSolutionFound);
    assert_eq!(result.reason_code, "SEARCH_SPACE_EXHAUSTED");
    assert_eq!(result.stats.candidates_tried, 0);
}

#[test]
fn invalid_search_config_is_rejected_before_search() {
    let products = [product("a", 1000.0)];
    let mut session = PlacementSession::new(&products).unwrap();
    let config = SearchConfig {
        candidate_buffer_size: 0,
        ..fast_config()
    };

    let result = search_placement(
        &mut session,
        &products,
        &config,
        &LayoutObjective::default(),
        1,
    );
    assert_eq!(result.status, Status::InvalidInput);
    assert_eq!(result.reason_code, "INVALID_SEARCH_CONFIG");
    assert_eq!(result.stats.candidates_tried, 0);
}
