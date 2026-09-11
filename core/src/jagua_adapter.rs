use jagua_rs::collision_detection::CDEConfig;
use jagua_rs::collision_detection::hazards::HazardEntity;
use jagua_rs::collision_detection::hazards::collector::BasicHazardCollector;
use jagua_rs::entities::{Container, Instance, Item, Layout, PlacedItem};
use jagua_rs::geometry::fail_fast::SPSurrogateConfig;
use jagua_rs::geometry::geo_enums::RotationRange;
use jagua_rs::geometry::primitives::{Point, Rect, SPolygon};
use jagua_rs::geometry::shape_modification::{ShapeModifyConfig, ShapeModifyMode};
use jagua_rs::geometry::{DTransformation, OriginalShape};
use jagua_rs::probs::bpp::entities::{
    BPInstance, BPLayoutType, BPPlacement, BPProblem, Bin, LayKey,
};

use crate::area::AREA_SIZE_MM;
use crate::error::ImportError;
use crate::geometry::Polygon;

/// jagua f32 geometri kullanır; f64 alan f32'ye adaptör sınırında çevrilir.
const AREA_F32: f32 = AREA_SIZE_MM as f32;

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

fn shape(vertices: &[[f64; 2]]) -> SPolygon {
    try_shape(vertices).expect("trusted proof geometry")
}

fn try_shape(vertices: &[[f64; 2]]) -> Result<SPolygon, ImportError> {
    SPolygon::new(
        vertices
            .iter()
            .map(|[x, y]| Point(*x as f32, *y as f32))
            .collect(),
    )
    .map_err(|_| ImportError::InvalidPolygon("jagua rejected polygon"))
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
    let container_shape = SPolygon::from(Rect::try_new(0.0, 0.0, AREA_F32, AREA_F32).unwrap());
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
        || bounds.x_max > AREA_F32
        || bounds.y_max > AREA_F32
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
    vertices: &Polygon,
    angle_radians: f64,
    translation: (f64, f64),
) -> PlacementProof {
    let item = item(0, shape(vertices));
    let mut layout = Layout::new(container());
    let d_transf = DTransformation::new(
        angle_radians as f32,
        (translation.0 as f32, translation.1 as f32),
    );
    let feasible = candidate_fits(&layout, &item, d_transf);
    if feasible {
        layout.place_item(&item, d_transf);
    }

    PlacementProof {
        continuous_rotation: item.allowed_rotation == RotationRange::Continuous,
        feasible,
    }
}

/// Tek kutulu yerleşim oturumu — plan §10. Bir adet 5000×5000 container,
/// ürün başına talep 1, collision geometrisi safety zone. Ürün kimliği ↔ item
/// indeks eşlemesi kararlıdır; büyükten küçüğe sıralama bu vektörü bozmaz
/// (sıralama ayrı yerleştirme sırası vektöründe yapılır, Aşama E).
pub struct PlacementSession {
    problem: BPProblem,
    /// item indeksi → ürün kimliği
    product_ids: Vec<String>,
    /// item indeksi → yerleşmiş poz (None: henüz yerleşmedi)
    placed: Vec<Option<crate::model::Pose>>,
    open_layout: Option<LayKey>,
    /// İlk yerleşimden önce sorgular için boş layout.
    empty_layout: Layout,
}

impl PlacementSession {
    pub fn new(products: &[crate::model::ProductGeometry]) -> Result<Self, ImportError> {
        if products.is_empty() {
            return Err(ImportError::InvalidPolygon(
                "session needs at least one product",
            ));
        }
        let mut product_ids = Vec::with_capacity(products.len());
        let mut items = Vec::with_capacity(products.len());
        for product in products {
            if product.safety_zone.len() < 3 {
                return Err(ImportError::InvalidPolygon("safety zone needs 3+ vertices"));
            }
            product_ids.push(product.id.clone());
            items.push((
                item(items.len(), try_shape(&product.safety_zone)?),
                1, // talep: ürün başına bir
            ));
        }
        let empty_layout = Layout::new(container());
        let problem = BPProblem::new(BPInstance::new(items, vec![Bin::new(container(), 1, 1)]));

        Ok(Self {
            product_ids,
            placed: vec![None; products.len()],
            open_layout: None,
            empty_layout,
            problem,
        })
    }

    /// item indeksi → ürün kimliği.
    pub fn product_id(&self, item_index: usize) -> &str {
        &self.product_ids[item_index]
    }

    /// Ürün kimliği → item indeksi (birebir eşleme).
    pub fn item_index_of(&self, product_id: &str) -> Option<usize> {
        self.product_ids.iter().position(|id| id == product_id)
    }

    /// Adayın tam poligon ve alan sınırı çakışma sorgusu (yerleştirmez).
    pub fn query_fit(&self, item_index: usize, pose: crate::model::Pose) -> bool {
        candidate_fits(
            self.current_layout(),
            self.problem.instance.item(item_index),
            d_transformation(pose),
        )
    }

    /// Geçerli adayı aynı yerleşime ekler. İkinci kutu ASLA açılmaz:
    /// çalışma zamanı kontrolü (release testi: `unfit_item_never_opens_a_second_bin`).
    pub fn try_place(&mut self, item_index: usize, pose: crate::model::Pose) -> bool {
        assert!(item_index < self.product_ids.len());
        if self.placed[item_index].is_some() || !self.query_fit(item_index, pose) {
            return false;
        }
        let layout_id = match self.open_layout {
            Some(key) => BPLayoutType::Open(key),
            None => BPLayoutType::Closed { bin_id: 0 },
        };
        let (lay_key, _) = self.problem.place_item(BPPlacement {
            layout_id,
            item_id: item_index,
            d_transf: d_transformation(pose),
        });
        match self.open_layout {
            None => self.open_layout = Some(lay_key),
            Some(key) => assert_eq!(lay_key, key, "second bin opened"),
        }
        self.placed[item_index] = Some(pose);
        true
    }

    /// Yerleşmiş pozlar, item indeksi sırasında.
    pub fn placements(&self) -> &[Option<crate::model::Pose>] {
        &self.placed
    }

    /// Kullanılan kutu sayısı: her zaman 1.
    pub fn bins_used(&self) -> usize {
        self.problem.bin_used_qtys().sum()
    }

    /// Tek kutu kuralının çalışma zamanı kontrolü.
    pub fn layout_count(&self) -> usize {
        self.problem.layouts.len()
    }

    /// Geri izleme: seçilmiş önceki pozlardan problem durumunu yeniden kurar
    /// (plan §10 — boş layout anahtarları yanlışlıkla yeniden kullanılmaz).
    pub fn restart_from(&mut self, placements: &[Option<crate::model::Pose>]) {
        assert_eq!(placements.len(), self.product_ids.len());
        self.problem = BPProblem::new(self.problem.instance.clone());
        self.open_layout = None;
        self.placed = vec![None; self.product_ids.len()];
        for (index, pose) in placements.iter().enumerate() {
            if let Some(pose) = *pose {
                assert!(
                    self.try_place(index, pose),
                    "restored placement no longer fits"
                );
            }
        }
    }

    fn current_layout(&self) -> &Layout {
        match self.open_layout {
            Some(key) => &self.problem.layouts[key],
            None => &self.empty_layout,
        }
    }
}

fn d_transformation(pose: crate::model::Pose) -> DTransformation {
    DTransformation::new(
        pose.rotation_rad as f32,
        (pose.x_mm as f32, pose.y_mm as f32),
    )
}

pub(crate) fn layout_proof(left: &Polygon, right: &Polygon) -> LayoutProof {
    let left = shape(left);
    let right = shape(right);
    let bounds = Rect::bounding_rect(left.bbox, right.bbox);
    let placement = DTransformation::new(
        0.0,
        (
            (AREA_F32 - bounds.width()) / 2.0 - bounds.x_min,
            (AREA_F32 - bounds.height()) / 2.0 - bounds.y_min,
        ),
    );
    let items = [
        left,
        right,
        shape(&[
            [0.0, 0.0],
            [f64::from(AREA_F32) + 1.0, 0.0],
            [f64::from(AREA_F32) + 1.0, f64::from(AREA_F32) + 1.0],
            [0.0, f64::from(AREA_F32) + 1.0],
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
