use groundscape_core::{PlacementRole, PlacementSession, Polygon, Pose, ProductGeometry};

fn product(id: &str, size: f64) -> ProductGeometry {
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
        placement_role: PlacementRole::Auto,
        tags: Vec::new(),
        age_group: None,
        source_metadata: None,
    }
}

#[test]
fn three_products_place_in_one_bin() {
    let products = [
        product("a", 1000.0),
        product("b", 800.0),
        product("c", 600.0),
    ];
    let mut session = PlacementSession::new(&products).unwrap();

    let poses = [
        Pose::new(100.0, 100.0, 0.0),
        Pose::new(1500.0, 100.0, 0.0),
        Pose::new(2500.0, 100.0, 0.0),
    ];
    for (index, pose) in poses.iter().enumerate() {
        assert!(session.try_place(index, *pose), "product {index} must fit");
    }

    assert_eq!(session.bins_used(), 1);
    assert_eq!(session.layout_count(), 1);
    for (index, pose) in session.placements().iter().enumerate() {
        assert!(pose.is_some(), "product {index} placed");
    }
}

#[test]
fn overlapping_pose_is_rejected_without_disturbing_placements() {
    let products = [product("a", 1000.0), product("b", 1000.0)];
    let mut session = PlacementSession::new(&products).unwrap();

    assert!(session.try_place(0, Pose::new(100.0, 100.0, 0.0)));
    // b'yi a'nın üzerine koymaya çalış
    assert!(!session.query_fit(1, Pose::new(500.0, 500.0, 0.0)));
    assert!(!session.try_place(1, Pose::new(500.0, 500.0, 0.0)));
    // a'nın pozu bozulmadı
    assert_eq!(session.placements()[0], Some(Pose::new(100.0, 100.0, 0.0)));
    assert_eq!(session.placements()[1], None);
}

#[test]
fn item_index_mapping_is_stable_and_bidirectional() {
    let products = [
        product("alpha", 1000.0),
        product("beta", 800.0),
        product("gamma", 600.0),
    ];
    let session = PlacementSession::new(&products).unwrap();

    assert_eq!(session.item_index_of("beta"), Some(1));
    assert_eq!(session.item_index_of("missing"), None);
    assert_eq!(session.product_id(2), "gamma");
}

#[test]
fn pose_round_trip_preserves_rotation_and_translation() {
    let products = [product("a", 600.0)];
    let mut session = PlacementSession::new(&products).unwrap();

    let pose = Pose::new(1200.0, 1800.0, 37.0_f64.to_radians());
    assert!(session.try_place(0, pose));
    assert!(session.placements()[0].is_some());
    let placed = session.placements()[0].unwrap();
    assert!((placed.x_mm - pose.x_mm).abs() <= 0.01);
    assert!((placed.y_mm - pose.y_mm).abs() <= 0.01);
    assert!((placed.rotation_rad - pose.rotation_rad).abs() <= 1e-4);
}

#[test]
fn unfit_item_never_opens_a_second_bin() {
    let products = [product("a", 4000.0), product("b", 4000.0)];
    let mut session = PlacementSession::new(&products).unwrap();

    assert!(session.try_place(0, Pose::new(500.0, 500.0, 0.0)));
    // ikinci 4000×4000 sığmaz; oturum yeni kutu açmaz
    assert!(!session.try_place(1, Pose::new(1000.0, 1000.0, 0.0)));
    assert_eq!(session.layout_count(), 1);
    assert_eq!(session.bins_used(), 1);
    assert_eq!(session.placements()[1], None);
}

#[test]
fn restart_from_rebuilds_same_state() {
    let products = [product("a", 1000.0), product("b", 800.0)];
    let mut session = PlacementSession::new(&products).unwrap();

    session.try_place(0, Pose::new(100.0, 100.0, 0.0));
    session.try_place(1, Pose::new(1500.0, 100.0, 0.0));
    let snapshot: Vec<_> = session.placements().to_vec();

    session.restart_from(&snapshot);
    assert_eq!(session.placements(), &snapshot);
    assert_eq!(session.bins_used(), 1);
    assert_eq!(session.layout_count(), 1);
}

#[test]
fn invalid_safety_zone_is_rejected() {
    let mut broken = product("a", 1000.0);
    broken.safety_zone = vec![[0.0, 0.0], [1.0, 0.0]];
    assert!(PlacementSession::new(std::slice::from_ref(&broken)).is_err());
}
