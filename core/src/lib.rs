mod jagua_adapter;

#[cfg(all(feature = "wasm", target_arch = "wasm32"))]
mod wasm_api;

use std::io::Cursor;

use dxf::{Drawing, entities::EntityType, enums::Units};
use serde::Serialize;

pub const AREA_SIZE: f32 = 5000.0;

pub const TINY_DXF: &[u8] = b"0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1015\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLWPOLYLINE\n100\nAcDbEntity\n8\n0\n100\nAcDbPolyline\n90\n4\n70\n1\n10\n0\n20\n0\n10\n100\n20\n0\n10\n100\n20\n100\n10\n0\n20\n100\n0\nENDSEC\n0\nEOF\n";

pub type Polygon = Vec<[f32; 2]>;

#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub angle_degrees: f32,
}

impl Pose {
    #[must_use]
    pub fn new(x: f32, y: f32, angle_degrees: f32) -> Self {
        Self {
            x,
            y,
            angle_degrees,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct DxfProof {
    pub units_mm: bool,
    pub closed_lwpolyline: bool,
    pub vertex_count: usize,
}

#[derive(Debug, Serialize)]
pub struct SpikeResult {
    pub engine: &'static str,
    pub dxf: DxfProof,
    pub overlap_detected: bool,
    pub continuous_rotation_enabled: bool,
    pub angle_37_accepted: bool,
    pub exact_fit_accepted: bool,
    pub out_of_area_rejected: bool,
    pub unfit_item_rejected: bool,
    pub area_size: [f32; 2],
    pub bins_used: usize,
}

#[must_use]
pub fn safety_polygons_overlap(left: &Polygon, right: &Polygon) -> bool {
    jagua_adapter::layout_proof(left, right).polygons_overlap
}

#[must_use]
pub fn jagua_pose_fits_fixed_area(polygon: &Polygon, pose: Pose) -> bool {
    jagua_adapter::placement_proof(polygon, pose.angle_degrees.to_radians(), (pose.x, pose.y))
        .feasible
}

pub fn parse_dxf_proof(bytes: &[u8]) -> Result<DxfProof, dxf::DxfError> {
    let drawing = Drawing::load(&mut Cursor::new(bytes))?;
    let polyline = drawing
        .entities()
        .find_map(|entity| match &entity.specific {
            EntityType::LwPolyline(polyline) => Some(polyline),
            _ => None,
        });

    Ok(DxfProof {
        units_mm: drawing.header.default_drawing_units == Units::Millimeters,
        closed_lwpolyline: polyline.is_some_and(dxf::entities::LwPolyline::is_closed),
        vertex_count: polyline.map_or(0, |polyline| polyline.vertices.len()),
    })
}

pub fn run_spike() -> Result<SpikeResult, dxf::DxfError> {
    let outer: Polygon = vec![[0.0, 0.0], [300.0, 0.0], [300.0, 300.0], [0.0, 300.0]];
    let inner: Polygon = vec![[50.0, 50.0], [100.0, 50.0], [100.0, 100.0], [50.0, 100.0]];
    let part: Polygon = vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]];
    let exact: Polygon = vec![
        [0.0, 0.0],
        [AREA_SIZE, 0.0],
        [AREA_SIZE, AREA_SIZE],
        [0.0, AREA_SIZE],
    ];
    let layout = jagua_adapter::layout_proof(&outer, &inner);
    let rotated = jagua_adapter::placement_proof(&part, 37.0_f32.to_radians(), (1000.0, 1000.0));

    Ok(SpikeResult {
        engine: "jagua-rs 0.8.1 BPP collision query",
        dxf: parse_dxf_proof(TINY_DXF)?,
        overlap_detected: layout.polygons_overlap,
        continuous_rotation_enabled: rotated.continuous_rotation,
        angle_37_accepted: rotated.feasible,
        exact_fit_accepted: jagua_pose_fits_fixed_area(&exact, Pose::new(0.0, 0.0, 0.0)),
        out_of_area_rejected: !jagua_pose_fits_fixed_area(&part, Pose::new(4950.0, 4950.0, 0.0)),
        unfit_item_rejected: layout.unfit_item_rejected,
        area_size: layout.area_size,
        bins_used: layout.bins_used,
    })
}
