//! Sürekli örneklemeli yerleşim araması — plan §11, Aşama E.
//!
//! Sıra: safety alanına göre büyükten küçüğe, eşitlikte ürün kimliği; sıra
//! yeniden başlatma ve geri izleme ile asla değişmez. Arama bütçesi
//! `SearchConfig`'tedir; toleranslar geometri politikasındadır.

use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde::Serialize;

use crate::area::{AREA_MM2, AREA_SIZE_MM};
use crate::geometry::{LINEAR_EPSILON_MM, Polygon, transformed_bbox};
use crate::jagua_adapter::PlacementSession;
use crate::model::{Placement, Pose, ProductGeometry};

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub global_samples_per_item: usize,
    pub local_samples_per_item: usize,
    pub candidate_buffer_size: usize,
    pub max_restarts: usize,
    pub max_total_candidates: usize,
}

impl Default for SearchConfig {
    /// Plan §11.3 başlangıç profili — kanıtlanmış performans değerleri değil,
    /// benchmark başlangıcı. `workerStepCandidates` adımlaması Aşama G'de
    /// mesaj döngüsüne aittir; burada tek çağrılı arama vardır.
    fn default() -> Self {
        Self {
            global_samples_per_item: 6000,
            local_samples_per_item: 2000,
            candidate_buffer_size: 6,
            max_restarts: 3,
            max_total_candidates: 200_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Status {
    Complete,
    Partial,
    NoSolutionFound,
    InvalidInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SearchStats {
    pub candidates_tried: usize,
    pub restarts: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlacementResult {
    pub status: Status,
    pub placements: Vec<Placement>,
    pub unplaced: Vec<String>,
    /// Hata sözlüğü kodu (plan §19): SEARCH_BUDGET_EXHAUSTED veya
    /// NECESSARY_AREA_EXCEEDED; başarılı sonuçta boş.
    pub reason_code: &'static str,
    pub stats: SearchStats,
}

/// Arama girişi. `products` oturum ürünleriyle aynı sıradadır (item indeksleri).
#[must_use]
pub fn search_placement(
    session: &mut PlacementSession,
    products: &[ProductGeometry],
    config: &SearchConfig,
    seed: u64,
) -> PlacementResult {
    assert_eq!(session.placements().len(), products.len());

    // Gerekli alan koşulu: ispatlanabilir ret, arama başlatılmaz (plan §11.1).
    let total_area: f64 = products.iter().map(|p| p.safety_area_mm2.abs()).sum();
    if total_area > AREA_MM2 {
        return PlacementResult {
            status: Status::InvalidInput,
            placements: vec![],
            unplaced: products.iter().map(|p| p.id.clone()).collect(),
            reason_code: "NECESSARY_AREA_EXCEEDED",
            stats: SearchStats {
                candidates_tried: 0,
                restarts: 0,
            },
        };
    }

    let order = order_by_safety_area(products);
    let n = products.len();
    // En derin ilerleyen durum saklanır: kısmi sonuç bundan raporlanır.
    let mut best = BestProgress {
        state: vec![None; n],
        candidates_tried: 0,
        restarts: 0,
    };

    for restart in 0..=config.max_restarts {
        let mut rng = StdRng::seed_from_u64(seed.wrapping_add(restart as u64));
        let mut budget = Budget::new(config.max_total_candidates);
        let mut state: Vec<Option<Pose>> = vec![None; n];
        // Temiz başlangıç: hiçbir yerleşim aktif değil.
        session.restart_from(&state);
        let snapshot = state.clone();
        if try_depth(
            session,
            &order,
            0,
            &mut state,
            &snapshot,
            products,
            config,
            &mut budget,
            &mut rng,
            &mut best,
            restart,
        ) {
            return PlacementResult {
                status: Status::Complete,
                placements: collect_placements(&order, &state, products),
                unplaced: vec![],
                reason_code: "",
                stats: SearchStats {
                    candidates_tried: budget.total,
                    restarts: restart,
                },
            };
        }
    }
    // Bütçe/arama tükendi: kısmi veya çözüm yok.
    let placements = collect_placements(&order, &best.state, products);
    let unplaced: Vec<String> = order
        .iter()
        .filter(|&&i| best.state[i].is_none())
        .map(|&i| products[i].id.clone())
        .collect();
    let status = if placements.is_empty() {
        Status::NoSolutionFound
    } else {
        Status::Partial
    };
    PlacementResult {
        status,
        placements,
        unplaced,
        reason_code: "SEARCH_BUDGET_EXHAUSTED",
        stats: SearchStats {
            candidates_tried: best.candidates_tried,
            restarts: best.restarts,
        },
    }
}

/// En derin ilerleyen arama durumu ve o duruma götüren restartın sayaçları.
struct BestProgress {
    state: Vec<Option<Pose>>,
    candidates_tried: usize,
    restarts: usize,
}

fn placed_count(state: &[Option<Pose>]) -> usize {
    state.iter().filter(|p| p.is_some()).count()
}

struct Budget {
    total: usize,
    max: usize,
}

impl Budget {
    fn new(max: usize) -> Self {
        Self { total: 0, max }
    }
}

/// Sıra: safety alanı azalan, eşitlikte ürün kimliği artan (plan §11.1).
fn order_by_safety_area(products: &[ProductGeometry]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..products.len()).collect();
    order.sort_by(|&a, &b| {
        let (x, y) = (&products[a], &products[b]);
        y.safety_area_mm2
            .abs()
            .partial_cmp(&x.safety_area_mm2.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| x.id.cmp(&y.id))
    });
    order
}

fn collect_placements(
    order: &[usize],
    state: &[Option<Pose>],
    products: &[ProductGeometry],
) -> Vec<Placement> {
    order
        .iter()
        .filter_map(|&i| {
            state[i].map(|pose| Placement {
                product_id: products[i].id.clone(),
                pose,
            })
        })
        .collect()
}

/// Derinlik öncelikli sınırlı geri izleme: ürün adayı üretir, yerleştirir;
/// alt ürün yerleşemezse bu ürünün sıradaki adayını dener (plan §11.3).
#[allow(clippy::too_many_arguments)]
fn try_depth(
    session: &mut PlacementSession,
    order: &[usize],
    depth: usize,
    state: &mut [Option<Pose>],
    snapshot_before_depth: &[Option<Pose>],
    products: &[ProductGeometry],
    config: &SearchConfig,
    budget: &mut Budget,
    rng: &mut StdRng,
    best: &mut BestProgress,
    restart: usize,
) -> bool {
    if depth == order.len() {
        // Son doğrulama orijinal geometri üzerinde bağımsız yapılır (plan §12);
        // geçersiz aday COMPLETE olamaz, arama adaylarıyla devam eder.
        let placements = collect_placements(order, state, products);
        return crate::validation::validate_result(products, &placements).valid;
    }
    let item = order[depth];
    let mut candidates = generate_candidates(session, item, &products[item], config, budget, rng);

    // Bottom-left benzeri tie-break: küçük x+y önce (plan §11.2).
    candidates.sort_by(|a, b| {
        (a.x_mm + a.y_mm)
            .partial_cmp(&(b.x_mm + b.y_mm))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for pose in candidates {
        if budget.total >= budget.max {
            break;
        }
        // Bu ürünün yerleşiminden önceki duruma dön (geri izlama yeniden kurulumu).
        session.restart_from(snapshot_before_depth);
        if session.try_place(item, pose) {
            let previous = state[item];
            state[item] = Some(pose);
            let child_snapshot: Vec<Option<Pose>> = state.to_vec();
            if try_depth(
                session,
                order,
                depth + 1,
                state,
                &child_snapshot,
                products,
                config,
                budget,
                rng,
                best,
                restart,
            ) {
                return true;
            }
            // Alt ürün yerleşemedi: en derin DOĞRULANMIŞ durumu kaydet, pozu geri al.
            let partial = collect_placements(order, state, products);
            if crate::validation::validate_result(products, &partial).valid
                && placed_count(state) > placed_count(&best.state)
            {
                best.state.clone_from_slice(state);
                best.candidates_tried = budget.total;
                best.restarts = restart;
            }
            state[item] = previous;
        }
    }
    // Oturumu bu derinliğin öncesi durumuna bırak.
    session.restart_from(snapshot_before_depth);
    false
}

/// Aday üretimi: 0/90/180/270 başlangıç açıları, sonra seed'li sürekli açı
/// örnekleri; konumlar kenar/köşe hizalı ve iç bölge örneklemesi karışık;
/// geçerli aday çevresinde küçülen pertürbasyonlar. Sıfır genişlikli aralıklar
/// rastgele dağılıma verilmez. Sayaç sınır nedeniyle erken elenen denemeleri de
/// kapsar (plan §11.2-11.3).
fn generate_candidates(
    session: &mut PlacementSession,
    item: usize,
    product: &ProductGeometry,
    config: &SearchConfig,
    budget: &mut Budget,
    rng: &mut StdRng,
) -> Vec<Pose> {
    let mut candidates: Vec<Pose> = Vec::new();
    let mut local_left = config.local_samples_per_item;
    let scale = product.safety_area_mm2.abs().sqrt().max(1.0);

    let mut attempts = 0usize;
    while candidates.len() < config.candidate_buffer_size
        && attempts < config.global_samples_per_item
        && budget.total < budget.max
    {
        // Açı: önce eksen hizalı başlangıçlar, sonra sürekli rastgele örnek.
        let angle = if attempts < 4 {
            f64::from(u32::try_from(attempts).unwrap_or(u32::MAX)) * std::f64::consts::FRAC_PI_2
        } else {
            rng.random_range(0.0..std::f64::consts::TAU)
        };
        attempts += 1;
        budget.total += 1;

        let Some((x_range, y_range)) = position_ranges(&product.safety_zone, angle) else {
            continue; // ters aralık: bu açı geçersiz
        };
        let position = if attempts.is_multiple_of(2) {
            // kenar/köşe hizalı başlangıç
            (x_range.0, y_range.0)
        } else {
            (sample_range(x_range, rng), sample_range(y_range, rng))
        };
        let pose = Pose::new(position.0, position.1, angle);
        if session.query_fit(item, pose) {
            // Küçülen pertürbasyonlarla yerel iyileştirme (bütçe dahilinde).
            perturb_around(
                session,
                item,
                pose,
                scale,
                &mut local_left,
                budget,
                rng,
                &mut candidates,
            );
            candidates.push(pose);
        }
    }
    candidates
}

#[allow(clippy::too_many_arguments)]
fn perturb_around(
    session: &mut PlacementSession,
    item: usize,
    base: Pose,
    scale: f64,
    local_left: &mut usize,
    budget: &mut Budget,
    rng: &mut StdRng,
    candidates: &mut Vec<Pose>,
) {
    // Ölçek: ürün boyutunun oranı; her denemede küçülür (plan §11.2).
    while *local_left > 0 && budget.total < budget.max {
        *local_left -= 1;
        budget.total += 1;
        let shrink = 1.0
            / f64::from(u32::try_from(*local_left).unwrap_or(1))
                .max(1.0)
                .sqrt();
        let dxy = scale * 0.25 * shrink;
        let dangle = 0.1 * shrink;
        let pose = Pose::new(
            base.x_mm + rng.random_range(-dxy..dxy),
            base.y_mm + rng.random_range(-dxy..dxy),
            base.rotation_rad + rng.random_range(-dangle..dangle),
        );
        if session.query_fit(item, pose) {
            candidates.push(pose);
        }
    }
}

/// Bir açı için döndürülmüş safety poligonunun sınırlarından geçerli çeviri
/// aralıkları. Ters aralık → None (geçersiz açı); sabit değer → tek nokta.
fn position_ranges(safety_zone: &Polygon, angle: f64) -> Option<((f64, f64), (f64, f64))> {
    let bbox = transformed_bbox(safety_zone, angle, [0.0; 2]);
    let (min_x, min_y, max_x, max_y) = (bbox[0], bbox[1], bbox[2], bbox[3]);
    let x_range = (-min_x, AREA_SIZE_MM - max_x);
    let y_range = (-min_y, AREA_SIZE_MM - max_y);
    if x_range.0 > x_range.1 || y_range.0 > y_range.1 {
        return None;
    }
    Some((x_range, y_range))
}

fn sample_range((lo, hi): (f64, f64), rng: &mut StdRng) -> f64 {
    if (hi - lo) <= LINEAR_EPSILON_MM {
        lo // sıfır genişlikli aralık rastgele dağılıma verilmez
    } else {
        rng.random_range(lo..hi)
    }
}
