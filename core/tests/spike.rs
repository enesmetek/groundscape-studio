use groundscape_core::{Polygon, Pose, TINY_DXF};
use groundscape_core::{
    jagua_pose_fits_fixed_area, parse_dxf_proof, run_spike, safety_polygons_overlap,
};

fn polygon(vertices: &[[f32; 2]]) -> Polygon {
    vertices.to_vec()
}

#[test]
fn separate_safety_polygons_do_not_overlap() {
    let left = polygon(&[[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]);
    let right = polygon(&[[200.0, 0.0], [300.0, 0.0], [300.0, 100.0], [200.0, 100.0]]);

    assert!(!safety_polygons_overlap(&left, &right));
}

#[test]
fn containment_overlaps() {
    let outer = polygon(&[[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]);
    let inner = polygon(&[[25.0, 25.0], [75.0, 25.0], [75.0, 75.0], [25.0, 75.0]]);

    assert!(safety_polygons_overlap(&outer, &inner));
}

#[test]
fn concave_shapes_with_overlapping_bounding_boxes_can_be_disjoint() {
    let lower_left = polygon(&[
        [0.0, 0.0],
        [4.0, 0.0],
        [4.0, 1.0],
        [1.0, 1.0],
        [1.0, 4.0],
        [0.0, 4.0],
    ]);
    let upper_right = polygon(&[
        [2.0, 2.0],
        [5.0, 2.0],
        [5.0, 5.0],
        [4.0, 5.0],
        [4.0, 3.0],
        [2.0, 3.0],
    ]);

    assert!(!safety_polygons_overlap(&lower_left, &upper_right));
}

#[test]
fn out_of_area_pose_is_rejected() {
    let part = polygon(&[[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]);

    for pose in [
        Pose::new(-0.1, 0.0, 0.0),
        Pose::new(0.0, -0.1, 0.0),
        Pose::new(4900.1, 0.0, 0.0),
        Pose::new(0.0, 4900.1, 0.0),
    ] {
        assert!(!jagua_pose_fits_fixed_area(&part, pose));
    }
}

#[test]
fn arbitrary_thirty_seven_degree_pose_is_accepted() {
    let part = polygon(&[[0.0, 0.0], [5500.0, 0.0], [5500.0, 500.0], [0.0, 500.0]]);

    assert!(!jagua_pose_fits_fixed_area(
        &part,
        Pose::new(400.0, 400.0, 0.0)
    ));
    assert!(jagua_pose_fits_fixed_area(
        &part,
        Pose::new(400.0, 400.0, 37.0)
    ));
    assert!(run_spike().unwrap().continuous_rotation_enabled);
}

#[test]
fn exact_5000_fit_is_accepted() {
    let part = polygon(&[[0.0, 0.0], [5000.0, 0.0], [5000.0, 5000.0], [0.0, 5000.0]]);

    assert!(jagua_pose_fits_fixed_area(&part, Pose::new(0.0, 0.0, 0.0)));
}

#[test]
fn spike_uses_one_fixed_area_without_a_second_bin() {
    let result = run_spike().unwrap();

    assert_eq!(result.area_size, [5000.0, 5000.0]);
    assert_eq!(result.bins_used, 1);
    assert!(result.unfit_item_rejected);
}

#[test]
fn controlled_dxf_fixture_parses_mm_closed_lwpolyline() {
    let proof = parse_dxf_proof(TINY_DXF).unwrap();

    assert!(proof.units_mm);
    assert!(proof.closed_lwpolyline);
    assert_eq!(proof.vertex_count, 4);
}
