use groundscape_core::{
    Placement, PlacementRole, Polygon, Pose, ProductGeometry, ValidationReport, validate_result,
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
        footprint_area_mm2: (size - 100.0) * (size - 100.0),
        footprint_centroid_local: [0.0, 0.0],
        placement_role: PlacementRole::Auto,
        tags: Vec::new(),
        age_group: None,
        source_metadata: None,
    }
}

fn placement(id: &str, x: f64, y: f64, deg: f64) -> Placement {
    Placement {
        product_id: id.to_owned(),
        pose: Pose::new(x, y, deg.to_radians()),
    }
}

#[test]
fn disjoint_placements_validate() {
    let products = [product("a", 1000.0), product("b", 1000.0)];
    let placements = [
        placement("a", 100.0, 100.0, 0.0),
        placement("b", 2000.0, 100.0, 0.0),
    ];

    let report = validate_result(&products, &placements);
    assert_eq!(
        report,
        ValidationReport {
            valid: true,
            issues: vec![]
        }
    );
}

#[test]
fn touching_safety_zones_are_valid() {
    let products = [product("a", 1000.0), product("b", 1000.0)];
    // Kenar teması: kesişim alanı sıfır (plan §2.2)
    let placements = [
        placement("a", 0.0, 0.0, 0.0),
        placement("b", 1000.0, 0.0, 0.0),
    ];

    assert!(validate_result(&products, &placements).valid);
}

#[test]
fn overlapping_safety_zones_rejected() {
    let products = [product("a", 1000.0), product("b", 1000.0)];
    let placements = [
        placement("a", 0.0, 0.0, 0.0),
        placement("b", 500.0, 0.0, 0.0),
    ];

    let report = validate_result(&products, &placements);
    assert_eq!(report.issues, ["ZONE_OVERLAP"]);
}

#[test]
fn full_containment_is_overlap() {
    // Bir poligon diğerinin tamamen içinde: yalnızca kenar testiyle kaçmaz
    let products = [product("outer", 4000.0), product("inner", 1000.0)];
    let placements = [
        placement("outer", 0.0, 0.0, 0.0),
        placement("inner", 1500.0, 1500.0, 0.0),
    ];

    let report = validate_result(&products, &placements);
    assert_eq!(report.issues, ["ZONE_OVERLAP"]);
}

#[test]
fn out_of_area_rejected() {
    let products = [product("a", 1000.0)];
    let placements = [placement("a", 4950.0, 100.0, 0.0)];

    let report = validate_result(&products, &placements);
    assert_eq!(report.issues, ["OUT_OF_AREA"]);
}

#[test]
fn footprint_outside_safety_rejected() {
    let mut broken = product("a", 1000.0);
    broken.footprint_polygons = vec![vec![
        [2000.0, 2000.0],
        [2100.0, 2000.0],
        [2100.0, 2100.0],
        [2000.0, 2100.0],
    ]];
    let placements = [placement("a", 0.0, 0.0, 0.0)];

    let report = validate_result(std::slice::from_ref(&broken), &placements);
    assert_eq!(report.issues, ["FOOTPRINT_OUTSIDE_SAFETY"]);
}

#[test]
fn scale_change_rejected() {
    let mut broken = product("a", 1000.0);
    broken.safety_area_mm2 = 500.0 * 500.0; // gerçek 1000² ile tutarsız
    let placements = [placement("a", 0.0, 0.0, 0.0)];

    let report = validate_result(std::slice::from_ref(&broken), &placements);
    assert_eq!(report.issues, ["SCALE_CHANGED"]);
}

#[test]
fn self_intersecting_polygon_rejected() {
    let mut broken = product("a", 1000.0);
    broken.safety_zone = vec![
        [0.0, 0.0],
        [1000.0, 1000.0], // öz-kesişen kum saati
        [1000.0, 0.0],
        [0.0, 1000.0],
    ];
    let placements = [placement("a", 0.0, 0.0, 0.0)];

    let report = validate_result(std::slice::from_ref(&broken), &placements);
    assert_eq!(report.issues, ["INVALID_POLYGON"]);
}

#[test]
fn unknown_and_duplicate_ids_rejected() {
    let products = [product("a", 1000.0)];
    let placements = [
        placement("ghost", 0.0, 0.0, 0.0),
        placement("a", 0.0, 0.0, 0.0),
        placement("a", 0.0, 0.0, 0.0),
    ];

    let report = validate_result(&products, &placements);
    assert!(report.issues.contains(&"UNKNOWN_PRODUCT"));
    assert!(report.issues.contains(&"DUPLICATE_PRODUCT"));
    assert!(!report.valid);
}

#[test]
fn rotated_pose_within_area_validates() {
    let products = [product("a", 1000.0)];
    let placements = [placement("a", 2000.0, 2000.0, 37.0)];

    assert!(validate_result(&products, &placements).valid);
}
