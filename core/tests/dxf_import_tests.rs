use groundscape_core::{
    FOOTPRINT_LAYER, ProductGeometry, SAFETY_ZONE_LAYER, TINY_DXF, import_product,
};

fn dxf_bytes(entities: &str, insunits: i32) -> Vec<u8> {
    format!(
        "0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n9\n$INSUNITS\n70\n{insunits}\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n{entities}0\nENDSEC\n0\nEOF\n"
    )
    .into_bytes()
}

fn lwpolyline(layer: &str, points: &[(&str, &str)], closed: bool) -> String {
    let mut out = String::from("0\nLWPOLYLINE\n100\nAcDbEntity\n8\n");
    out.push_str(layer);
    out.push_str("\n100\nAcDbPolyline\n90\n");
    out.push_str(&points.len().to_string());
    out.push_str("\n70\n");
    out.push_str(if closed { "1" } else { "0" });
    for (x, y) in points {
        out.push_str(&format!("\n10\n{x}\n20\n{y}"));
    }
    out.push('\n');
    out
}

fn arc(layer: &str) -> String {
    format!(
        "0\nARC\n100\nAcDbEntity\n8\n{layer}\n100\nAcDbCircle\n10\n0\n20\n0\n30\n0\n40\n100\n50\n0\n51\n90\n"
    )
}

fn square_fp() -> String {
    lwpolyline(
        FOOTPRINT_LAYER,
        &[("0", "0"), ("100", "0"), ("100", "100"), ("0", "100")],
        true,
    )
}

fn square_sz(size: &str) -> String {
    lwpolyline(
        SAFETY_ZONE_LAYER,
        &[("0", "0"), (size, "0"), (size, size), ("0", size)],
        true,
    )
}

fn valid_product_dxf() -> Vec<u8> {
    dxf_bytes(&format!("{}{}", square_fp(), square_sz("200")), 4)
}

#[test]
fn valid_fixture_imports_with_canonical_geometry() {
    let product = import_product("p1", &valid_product_dxf()).unwrap();
    assert_eq!(product.id, "p1");
    assert_eq!(product.footprint_polygons.len(), 1);
    // Safety 200x200, bbox merkezi (100,100); her iki katmandan çıkarılır
    assert_eq!(product.footprint_polygons[0][0], [-100.0, -100.0]);
    assert_eq!(product.safety_zone[0], [-100.0, -100.0]);
    assert_eq!(product.safety_area_mm2, 40_000.0);
}

#[test]
fn dxf_parse_failure_maps_to_dictionary_code() {
    let err = import_product("p1", b"not a dxf").unwrap_err();
    assert_eq!(err.code(), "DXF_PARSE_FAILED");
}

#[test]
fn unitless_is_unspecified_and_other_units_rejected() {
    let err = import_product(
        "p1",
        &dxf_bytes(&format!("{}{}", square_fp(), square_sz("200")), 0),
    )
    .unwrap_err();
    assert_eq!(err.code(), "UNIT_UNSPECIFIED");

    let err = import_product(
        "p1",
        &dxf_bytes(&format!("{}{}", square_fp(), square_sz("200")), 1),
    )
    .unwrap_err();
    assert_eq!(err.code(), "UNIT_NOT_SUPPORTED");
}

#[test]
fn missing_layers_rejected() {
    let err = import_product("p1", &dxf_bytes(&square_fp(), 4)).unwrap_err();
    assert_eq!(err.code(), "MISSING_LAYER");

    let err = import_product("p1", &dxf_bytes(&square_sz("200"), 4)).unwrap_err();
    assert_eq!(err.code(), "MISSING_LAYER");
}

#[test]
fn unsupported_entity_in_selected_layer_rejected() {
    let dxf = dxf_bytes(
        &format!(
            "{}{}{}",
            square_fp(),
            arc(FOOTPRINT_LAYER),
            square_sz("200")
        ),
        4,
    );
    let err = import_product("p1", &dxf).unwrap_err();
    assert_eq!(err.code(), "UNSUPPORTED_ENTITY");

    // Diğer katmandaki entity sessizce yok sayılır
    let dxf = dxf_bytes(
        &format!("{}{}{}", arc("OTHER"), square_fp(), square_sz("200")),
        4,
    );
    assert!(import_product("p1", &dxf).is_ok());
}

#[test]
fn open_and_bulged_contours_rejected() {
    let dxf = dxf_bytes(
        &format!(
            "{}{}",
            lwpolyline(
                FOOTPRINT_LAYER,
                &[("0", "0"), ("100", "0"), ("100", "100")],
                false
            ),
            square_sz("200")
        ),
        4,
    );
    let err = import_product("p1", &dxf).unwrap_err();
    assert_eq!(err.code(), "OPEN_CONTOUR");

    let bulged = format!(
        "0\nLWPOLYLINE\n100\nAcDbEntity\n8\n{}\n100\nAcDbPolyline\n90\n4\n70\n1\n10\n0\n20\n0\n10\n100\n20\n0\n42\n0.5\n10\n100\n20\n100\n10\n0\n20\n100\n",
        FOOTPRINT_LAYER
    );
    let dxf = dxf_bytes(&format!("{}{}", bulged, square_sz("200")), 4);
    let err = import_product("p1", &dxf).unwrap_err();
    assert_eq!(err.code(), "UNSUPPORTED_ENTITY");
}

#[test]
fn two_safety_contours_rejected() {
    let dxf = dxf_bytes(
        &format!("{}{}{}", square_fp(), square_sz("200"), square_sz("200")),
        4,
    );
    let err = import_product("p1", &dxf).unwrap_err();
    assert_eq!(err.code(), "INVALID_POLYGON");
}

#[test]
fn footprint_edge_outside_safety_rejected() {
    // Köşeler içeride ama kenar konkav çentikten dışarı çıkar
    let notched_sz = format!(
        "0\nLWPOLYLINE\n100\nAcDbEntity\n8\n{}\n100\nAcDbPolyline\n90\n8\n70\n1\n10\n0\n20\n0\n10\n100\n20\n0\n10\n100\n20\n100\n10\n60\n20\n100\n10\n60\n20\n60\n10\n40\n20\n60\n10\n40\n20\n100\n10\n0\n20\n100\n",
        SAFETY_ZONE_LAYER
    );
    let dxf = dxf_bytes(&format!("{}{}", square_fp(), notched_sz), 4);
    let err = import_product("p1", &dxf).unwrap_err();
    assert_eq!(err.code(), "FOOTPRINT_OUTSIDE_SAFETY");
}

#[test]
fn spike_fixture_still_parses() {
    assert!(import_product("legacy", TINY_DXF).is_err()); // katmanı yok
}

#[test]
fn source_metadata_records_offset() {
    let product = import_product("p1", &valid_product_dxf()).unwrap();
    assert!(
        product
            .source_metadata
            .as_ref()
            .is_some_and(|m| m.contains("offset="))
    );
    let _ = std::mem::size_of::<ProductGeometry>();
}

#[test]
fn import_computes_footprint_area_and_centroid() {
    // Asimetrik tampon: footprint 100x100, safety içinde sola/alta 50,
    // sağa/yukarıya 150 boşluk — canonical merkez ≠ (0,0) (plan P1 kapısı).
    let offcenter_fp = lwpolyline(
        FOOTPRINT_LAYER,
        &[("50", "50"), ("150", "50"), ("150", "150"), ("50", "150")],
        true,
    );
    let dxf = dxf_bytes(&format!("{}{}", offcenter_fp, square_sz("300")), 4);

    let product = import_product("p1", &dxf).unwrap();
    assert_eq!(product.footprint_area_mm2, 10_000.0);
    assert_eq!(product.footprint_centroid_local, [-50.0, -50.0]);
    assert_eq!(
        product.placement_role,
        groundscape_core::PlacementRole::Auto
    );
    assert!(product.tags.is_empty());
    assert_eq!(product.age_group, None);
}

#[test]
fn multipart_footprint_centroid_is_area_weighted() {
    // Parça 1: 100x100 (alan 10k) merkez (50,50); parça 2: 200x100 (alan 20k)
    // merkez (300,50) → ağırlıklı merkez x = (10k·50 + 20k·300)/30k = 650/3.
    let part1 = lwpolyline(
        FOOTPRINT_LAYER,
        &[("0", "0"), ("100", "0"), ("100", "100"), ("0", "100")],
        true,
    );
    let part2 = lwpolyline(
        FOOTPRINT_LAYER,
        &[("200", "0"), ("400", "0"), ("400", "100"), ("200", "100")],
        true,
    );
    let dxf = dxf_bytes(&format!("{}{}{}", part1, part2, square_sz("500")), 4);

    let product = import_product("p1", &dxf).unwrap();
    assert_eq!(product.footprint_area_mm2, 30_000.0);
    let [cx, cy] = product.footprint_centroid_local;
    assert!((cx - (650.0 / 3.0 - 250.0)).abs() < 1e-6, "cx={cx}");
    assert!((cy - (-200.0)).abs() < 1e-6, "cy={cy}");
}
