//! Sürekli örneklemeli yerleşim araması — plan §11, Aşama E.
//!
//! Sıra: safety alanına göre büyükten küçüğe, eşitlikte ürün kimliği; sıra
//! yeniden başlatma ve geri izleme ile asla değişmez. Arama bütçesi
//! `SearchConfig`'tedir; toleranslar geometri politikasındadır.

use rand::{RngExt, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};

use crate::area::{AREA_MM2, AREA_SIZE_MM};
use crate::geometry::{LINEAR_EPSILON_MM, Polygon, transformed_bbox};
use crate::jagua_adapter::PlacementSession;
use crate::model::{Placement, PlacementRole, Pose, ProductGeometry};
use crate::scoring::{
    LayoutObjective, ScoredItem, effective_role, score_layout, score_layout_delta,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Complete,
    Partial,
    NoSolutionFound,
    InvalidInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchStats {
    pub candidates_tried: usize,
    pub restarts: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
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
    objective: &LayoutObjective,
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
    let largest_footprint = products
        .iter()
        .map(|p| p.footprint_area_mm2.abs())
        .fold(0.0_f64, f64::max);
    // En derin ilerleyen durum saklanır: kısmi sonuç bundan raporlanır.
    // Eşit derinlikte düşük toplam skor ikincil anahtardır (MVP-2 plan P4).
    let mut best = BestProgress {
        state: vec![None; n],
        score: f64::INFINITY,
    };
    let mut budget = Budget::new(config.max_total_candidates);
    let mut last_restart = 0;

    for restart in 0..=config.max_restarts {
        if budget.total >= budget.max {
            break;
        }
        last_restart = restart;
        let mut rng = StdRng::seed_from_u64(seed.wrapping_add(restart as u64));
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
            largest_footprint,
            objective,
            config,
            &mut budget,
            &mut rng,
            &mut best,
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
            candidates_tried: budget.total,
            restarts: last_restart,
        },
    }
}

/// En derin ilerleyen arama durumu ve o duruma götüren restartın sayaçları.
/// Yerleşen ürün sayısı birincil anahtar (geçerlilik önce gelir); eşit sayıda
/// düşük toplam skor ikincil anahtar (MVP-2 plan P4).
struct BestProgress {
    state: Vec<Option<Pose>>,
    score: f64,
}

fn placed_count(state: &[Option<Pose>]) -> usize {
    state.iter().filter(|p| p.is_some()).count()
}

/// Skorlama girişi: yerleşmiş ürünlerin `ScoredItem` listesi (plan P2 —
/// `placed` mevcut state'ten türetilir, ayrı durum saklanmaz).
fn placed_scored_items(
    state: &[Option<Pose>],
    products: &[ProductGeometry],
    largest_footprint: f64,
) -> Vec<ScoredItem> {
    state
        .iter()
        .enumerate()
        .filter_map(|(item, pose)| {
            let product = &products[item];
            pose.map(|pose| {
                ScoredItem::new(
                    pose,
                    product.footprint_centroid_local,
                    product.footprint_area_mm2,
                    product.placement_role,
                    largest_footprint,
                )
            })
        })
        .collect()
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
    largest_footprint: f64,
    objective: &LayoutObjective,
    config: &SearchConfig,
    budget: &mut Budget,
    rng: &mut StdRng,
    best: &mut BestProgress,
) -> bool {
    if depth == order.len() {
        // Son doğrulama orijinal geometri üzerinde bağımsız yapılır (plan §12);
        // geçersiz aday COMPLETE olamaz, arama adaylarıyla devam eder.
        let placements = collect_placements(order, state, products);
        return crate::validation::validate_result(products, &placements).valid;
    }
    let item = order[depth];
    let mut candidates = generate_candidates(
        session,
        item,
        &products[item],
        largest_footprint,
        objective,
        config,
        budget,
        rng,
    );

    // Skor sıralaması (MVP-2 plan P4): bottom-left tie-break kaldırıldı.
    // Düşük skor önce; `placed`, state + products'tan bu derinlikte türetilir.
    let placed = placed_scored_items(state, products, largest_footprint);
    let candidate_item = |pose: &Pose| {
        let product = &products[item];
        ScoredItem::new(
            *pose,
            product.footprint_centroid_local,
            product.footprint_area_mm2,
            product.placement_role,
            largest_footprint,
        )
    };
    candidates.sort_by(|a, b| {
        score_layout_delta(candidate_item(a), &placed, objective)
            .partial_cmp(&score_layout_delta(candidate_item(b), &placed, objective))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for pose in candidates {
        if budget.total >= budget.max {
            break;
        }
        budget.total += 1;
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
                largest_footprint,
                objective,
                config,
                budget,
                rng,
                best,
            ) {
                return true;
            }
            // Alt ürün yerleşemedi: en derin DOĞRULANMIŞ durumu kaydet, pozu geri al.
            let partial = collect_placements(order, state, products);
            let partial_score = score_layout(
                &placed_scored_items(state, products, largest_footprint),
                objective,
            );
            if crate::validation::validate_result(products, &partial).valid
                && (placed_count(state) > placed_count(&best.state)
                    || (placed_count(state) == placed_count(&best.state)
                        && partial_score < best.score))
            {
                best.state.clone_from_slice(state);
                best.score = partial_score;
            }
            state[item] = previous;
        }
    }
    // Oturumu bu derinliğin öncesi durumuna bırak.
    session.restart_from(snapshot_before_depth);
    false
}

/// Aday üretimi (MVP-2 plan P3): iki havuz.
///
/// Genel havuz — rol-farkındalıklı bölge örneklemesi: anchor/auto-büyük merkezi
/// bölge çevresinden, peripheral köşe hizalı kenarlardan, diğerleri alanın
/// farklı kesimlerinden. Tampon, çeşitlilik koşuluyla doldurulur: yeni aday
/// mevcut adayların en yakınına en az `0.25·√safety_area` uzaklıkta olmalı —
/// aynı konumun mm-varyasyonları tamponu dolduramaz.
///
/// Yerel havuz (`perturb_around`) yalnızca farklı bölge adayları denendikten
/// sonra, tamponun kalan slotları için devreye girer; küçülen pertürbasyonla
/// yerel iyileştirme yapar (plan §11.2).
#[allow(clippy::too_many_arguments)]
fn generate_candidates(
    session: &mut PlacementSession,
    item: usize,
    product: &ProductGeometry,
    largest_footprint: f64,
    objective: &LayoutObjective,
    config: &SearchConfig,
    budget: &mut Budget,
    rng: &mut StdRng,
) -> Vec<Pose> {
    let mut candidates: Vec<Pose> = Vec::new();
    let scale = product.safety_area_mm2.abs().sqrt().max(1.0);
    let diversity_min = 0.25 * scale;
    let role = effective_role(
        product.placement_role,
        product.footprint_area_mm2.abs(),
        largest_footprint,
    );

    // Genel havuz: bölgesel örneklem + çeşitlilik koşulu.
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
        let position = match role {
            PlacementRole::Anchor => {
                // Merkezi bölge çevresinden örnek; geçerli aralığa kırpılır.
                let half = objective.anchor_region_ratio * AREA_SIZE_MM;
                let center = AREA_SIZE_MM / 2.0;
                (
                    sample_range(
                        (
                            (center - half).max(x_range.0),
                            (center + half).min(x_range.1),
                        ),
                        rng,
                    ),
                    sample_range(
                        (
                            (center - half).max(y_range.0),
                            (center + half).min(y_range.1),
                        ),
                        rng,
                    ),
                )
            }
            PlacementRole::Peripheral => {
                // Farklı kenar/köşelerden hizalı başlangıçlar (döngüsel).
                let corner = attempts % 4;
                (
                    if corner & 1 == 0 {
                        x_range.0
                    } else {
                        x_range.1
                    },
                    if corner & 2 == 0 {
                        y_range.0
                    } else {
                        y_range.1
                    },
                )
            }
            _ => {
                if attempts.is_multiple_of(2) {
                    // kenar/köşe hizalı başlangıç
                    (x_range.0, y_range.0)
                } else {
                    (sample_range(x_range, rng), sample_range(y_range, rng))
                }
            }
        };
        let pose = Pose::new(position.0, position.1, angle);
        if session.query_fit(item, pose) && is_diverse(pose, &candidates, diversity_min) {
            candidates.push(pose);
        }
    }

    // Yerel havuz: kalan slotlar için küçülen pertürbasyonlar.
    if let Some(&base) = candidates.last() {
        let mut local_left = config.local_samples_per_item;
        perturb_around(
            session,
            item,
            base,
            scale,
            &mut local_left,
            config.local_samples_per_item,
            budget,
            rng,
            &mut candidates,
            config.candidate_buffer_size,
        );
    }
    candidates
}

/// Çeşitlilik koşulu: aday, mevcut her adayın konumuna eşikten uzak olmalı.
fn is_diverse(pose: Pose, candidates: &[Pose], threshold: f64) -> bool {
    candidates.iter().all(|c| {
        (c.x_mm - pose.x_mm).powi(2) + (c.y_mm - pose.y_mm).powi(2) >= threshold * threshold
    })
}

#[allow(clippy::too_many_arguments)]
fn perturb_around(
    session: &mut PlacementSession,
    item: usize,
    base: Pose,
    scale: f64,
    local_left: &mut usize,
    local_total: usize,
    budget: &mut Budget,
    rng: &mut StdRng,
    candidates: &mut Vec<Pose>,
    candidate_buffer_size: usize,
) {
    // Ölçek: ürün boyutunun oranı; kalan örnek sayısına göre küçülür
    // (MVP-2 plan P3 düzeltmesi — pertürbasyon monoton küçülür).
    while *local_left > 0
        && budget.total < budget.max
        && candidates.len() + 1 < candidate_buffer_size
    {
        *local_left -= 1;
        budget.total += 1;
        let shrink = perturb_scale(*local_left, local_total);
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

/// Pertürbasyon ölçeği: kalan yerel örnek sayısı azaldıkça küçülür.
/// Eski `1/√local_left` formu ters çalışıyordu (MVP-2 plan P3).
fn perturb_scale(local_left: usize, local_total: usize) -> f64 {
    (local_left as f64 / local_total.max(1) as f64).sqrt()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::PlacementRole;

    fn product(id: &str, size: f64, role: PlacementRole) -> ProductGeometry {
        let safety: Polygon = vec![[0.0, 0.0], [size, 0.0], [size, size], [0.0, size]];
        let half = size / 2.0;
        let footprint: Polygon = vec![
            [half, half],
            [size - 10.0, half],
            [size - 10.0, size - 10.0],
            [half, size - 10.0],
        ];
        ProductGeometry {
            id: id.to_owned(),
            footprint_polygons: vec![footprint],
            safety_zone: safety,
            safety_area_mm2: size * size,
            footprint_area_mm2: (size - 10.0 - half) * (size - 10.0 - half),
            footprint_centroid_local: [0.0, 0.0],
            placement_role: role,
            tags: Vec::new(),
            age_group: None,
            source_metadata: None,
        }
    }

    #[test]
    fn perturb_scale_decreases_monotonically() {
        let total = 10;
        let mut prev = f64::INFINITY;
        for left in (1..=total).rev() {
            let s = perturb_scale(left, total);
            assert!(s < prev, "scale not decreasing: left={left} s={s}");
            prev = s;
        }
    }

    #[test]
    fn anchor_candidates_sample_the_center_region() {
        let products = [product("big", 1200.0, PlacementRole::Anchor)];
        let mut session = PlacementSession::new(&products).unwrap();
        let config = SearchConfig::default();
        let objective = LayoutObjective::default();
        let mut budget = Budget::new(config.max_total_candidates);
        let mut rng = StdRng::seed_from_u64(11);

        let cands = generate_candidates(
            &mut session,
            0,
            &products[0],
            1_440_000.0,
            &objective,
            &config,
            &mut budget,
            &mut rng,
        );
        assert!(!cands.is_empty());
        let half = objective.anchor_region_ratio * AREA_SIZE_MM;
        for pose in &cands {
            // Merkezi bölge: alan merkezinden ±0.25·5000.
            assert!(
                (pose.x_mm - AREA_SIZE_MM / 2.0).abs() <= half + 1e-6,
                "x={} outside center region",
                pose.x_mm
            );
            assert!(
                (pose.y_mm - AREA_SIZE_MM / 2.0).abs() <= half + 1e-6,
                "y={} outside center region",
                pose.y_mm
            );
        }
    }

    #[test]
    fn peripheral_candidates_hit_different_corners() {
        let products = [product("edge", 800.0, PlacementRole::Peripheral)];
        let mut session = PlacementSession::new(&products).unwrap();
        let config = SearchConfig::default();
        let objective = LayoutObjective::default();
        let mut budget = Budget::new(config.max_total_candidates);
        let mut rng = StdRng::seed_from_u64(13);

        let cands = generate_candidates(
            &mut session,
            0,
            &products[0],
            100.0,
            &objective,
            &config,
            &mut budget,
            &mut rng,
        );
        // Kenar/köşe hizalı: adayların en az yarısı alan kenarına dayalı.
        let on_edge = cands
            .iter()
            .filter(|p| {
                p.x_mm < 1e-6
                    || p.y_mm < 1e-6
                    || p.x_mm > AREA_SIZE_MM - 800.0 - 1e-6
                    || p.y_mm > AREA_SIZE_MM - 800.0 - 1e-6
            })
            .count();
        assert!(on_edge >= cands.len() / 2, "on_edge={on_edge}");
    }

    #[test]
    fn buffer_respects_diversity_threshold() {
        // local havuz kapalı: tüm adaylar genel havuzdan, çeşitlilik koşullu.
        let products = [product("mid", 900.0, PlacementRole::Distributed)];
        let mut session = PlacementSession::new(&products).unwrap();
        let config = SearchConfig {
            local_samples_per_item: 0,
            ..SearchConfig::default()
        };
        let objective = LayoutObjective::default();
        let mut budget = Budget::new(config.max_total_candidates);
        let mut rng = StdRng::seed_from_u64(17);

        let cands = generate_candidates(
            &mut session,
            0,
            &products[0],
            100.0,
            &objective,
            &config,
            &mut budget,
            &mut rng,
        );
        let threshold = 0.25 * 900.0;
        for (i, a) in cands.iter().enumerate() {
            for b in cands.iter().skip(i + 1) {
                let d = ((a.x_mm - b.x_mm).powi(2) + (a.y_mm - b.y_mm).powi(2)).sqrt();
                assert!(d >= threshold - 1e-6, "too close: {a:?} vs {b:?}");
            }
        }
    }
}
