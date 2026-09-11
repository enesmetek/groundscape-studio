use jagua_rs::collision_detection::CDEConfig;
use jagua_rs::collision_detection::hazards::HazardEntity;
use jagua_rs::collision_detection::hazards::collector::BasicHazardCollector;
use jagua_rs::entities::{Container, Instance, Item, Layout, PlacedItem};
use jagua_rs::geometry::fail_fast::SPSurrogateConfig;
use jagua_rs::geometry::geo_enums::RotationRange;
use jagua_rs::geometry::primitives::{Point, Rect, SPolygon};
use jagua_rs::geometry::shape_modification::{ShapeModifyConfig, ShapeModifyMode};
use jagua_rs::geometry::{DTransformation, OriginalShape};
use jagua_rs::probs::bpp::entities::{BPInstance, BPLayoutType, BPPlacement, BPProblem, Bin};

use crate::AREA_SIZE;

pub(crate) struct LayoutProof {
    pub(crate) area_size: [f32; 2],
    pub(crate) bins_used: usize,
    pub(crate) polygons_overlap: bool,
    pub(crate) unfit_item_rejected: bool,
}

pub(crate) struct PlacementProof {
    pub(crate) continuous_rotation: bool,
    pub(crate) feasible: bool,
}

fn shape(vertices: &[[f32; 2]]) -> SPolygon {
    SPolygon::new(vertices.iter().map(|[x, y]| Point(*x, *y)).collect()).unwrap()
}

fn original(shape: SPolygon, mode: ShapeModifyMode) -> OriginalShape {
    OriginalShape {
        shape,
        pre_transform: DTransformation::empty(),
        modify_mode: mode,
        modify_config: ShapeModifyConfig::default(),
    }
}

fn container() -> Container {
    let container_shape = SPolygon::from(Rect::try_new(0.0, 0.0, AREA_SIZE, AREA_SIZE).unwrap());
    Container::new(
        0,
        original(container_shape, ShapeModifyMode::Deflate),
        vec![],
        CDEConfig {
            quadtree_depth: 4,
            cd_threshold: 4,
            item_surrogate_config: SPSurrogateConfig::none(),
        },
    )
    .unwrap()
}

fn item(id: usize, shape: SPolygon) -> Item {
    Item::new(
        id,
        original(shape, ShapeModifyMode::Inflate),
        RotationRange::Continuous,
        None,
        SPSurrogateConfig::none(),
    )
    .unwrap()
}

fn candidate_fits(layout: &Layout, item: &Item, d_transf: DTransformation) -> bool {
    let candidate = PlacedItem::new(item, d_transf);
    let bounds = candidate.shape.bbox;
    if bounds.x_min < 0.0
        || bounds.y_min < 0.0
        || bounds.x_max > AREA_SIZE
        || bounds.y_max > AREA_SIZE
    {
        return false;
    }

    let mut collisions = BasicHazardCollector::new();
    layout
        .cde()
        .collect_poly_collisions(&candidate.shape, &mut collisions);
    collisions
        .iter()
        .all(|(_, entity)| matches!(entity, HazardEntity::Exterior))
}

pub(crate) fn placement_proof(
    vertices: &[[f32; 2]],
    angle_radians: f32,
    translation: (f32, f32),
) -> PlacementProof {
    let item = item(0, shape(vertices));
    let mut layout = Layout::new(container());
    let d_transf = DTransformation::new(angle_radians, translation);
    let feasible = candidate_fits(&layout, &item, d_transf);
    if feasible {
        layout.place_item(&item, d_transf);
    }

    PlacementProof {
        continuous_rotation: item.allowed_rotation == RotationRange::Continuous,
        feasible,
    }
}

pub(crate) fn layout_proof(left: &[[f32; 2]], right: &[[f32; 2]]) -> LayoutProof {
    let left = shape(left);
    let right = shape(right);
    let bounds = Rect::bounding_rect(left.bbox, right.bbox);
    let placement = DTransformation::new(
        0.0,
        (
            (AREA_SIZE - bounds.width()) / 2.0 - bounds.x_min,
            (AREA_SIZE - bounds.height()) / 2.0 - bounds.y_min,
        ),
    );
    let items = [
        left,
        right,
        shape(&[
            [0.0, 0.0],
            [AREA_SIZE + 1.0, 0.0],
            [AREA_SIZE + 1.0, AREA_SIZE + 1.0],
            [0.0, AREA_SIZE + 1.0],
        ]),
    ]
    .into_iter()
    .enumerate()
    .map(|(id, shape)| item(id, shape))
    .map(|item| (item, 1))
    .collect();
    let mut problem = BPProblem::new(BPInstance::new(items, vec![Bin::new(container(), 1, 1)]));
    let empty_layout = Layout::new(problem.instance.bins[0].container.clone());
    assert!(candidate_fits(
        &empty_layout,
        problem.instance.item(0),
        placement
    ));
    let (layout, _) = problem.place_item(BPPlacement {
        layout_id: BPLayoutType::Closed { bin_id: 0 },
        item_id: 0,
        d_transf: placement,
    });
    let polygons_overlap = !candidate_fits(
        &problem.layouts[layout],
        problem.instance.item(1),
        placement,
    );
    if !polygons_overlap {
        problem.place_item(BPPlacement {
            layout_id: BPLayoutType::Open(layout),
            item_id: 1,
            d_transf: placement,
        });
    }
    let unfit_item_rejected = !candidate_fits(
        &problem.layouts[layout],
        problem.instance.item(2),
        DTransformation::empty(),
    );

    let area = problem.instance.bins[0].container.outer_orig.bbox();

    LayoutProof {
        area_size: [area.width(), area.height()],
        bins_used: problem.bin_used_qtys().sum(),
        polygons_overlap,
        unfit_item_rejected: unfit_item_rejected
            && problem.item_demand_qtys[2] == 1
            && problem.layouts.len() == 1,
    }
}
