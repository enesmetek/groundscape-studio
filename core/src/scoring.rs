//! Yerleşim skoru — MVP-2 plan P2 (tasarım §4.4). "İlk geçerli konumu kabul
//! et" yerine "geçerli çözümler arasında en iyi skorluyu seç" hedefinin
//! ölçütü. Düşük skor daha iyidir; tüm bileşenler alan boyutuna
//! (`AREA_SIZE_MM`) normalize edilir, sabit mm eşikleri kullanılmaz.
//!
//! Skor YALNIZCA geçerli çözümler arasında seçim yapar; güvenlik asla
//! gevşetilmez (plan §2.8). Deterministiktir: aynı girdi, aynı puan.

use serde::{Deserialize, Serialize};

use crate::area::AREA_SIZE_MM;
use crate::model::{PlacementRole, Pose};

/// Skor bileşen ağırlıkları.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentWeights {
    pub anchor: f64,
    pub balance: f64,
    pub distribution: f64,
    pub spacing: f64,
    pub orientation: f64,
}

impl Default for ComponentWeights {
    /// Ölçülmemiş başlangıç: yönlenme dışında hepsi 1.0 (plan P2 — ağırlık
    /// ölçülmeden zorlanmaz, P5'te ayarlanır).
    fn default() -> Self {
        Self {
            anchor: 1.0,
            balance: 1.0,
            distribution: 1.0,
            spacing: 1.0,
            orientation: 0.0,
        }
    }
}

/// Skorlama hedefi. `SearchConfig`'e gömülmez; ayrı kavram (plan P4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutObjective {
    pub weights: ComponentWeights,
    /// Bölgesel dağılım grid'i (grid×grid).
    pub grid: u32,
    /// "Merkezi bölge" yarı genişliği, alan boyutuna oranla.
    pub anchor_region_ratio: f64,
    /// Komşuluk hedef boşluğu, alan boyutuna oranla.
    pub spacing_target_ratio: f64,
}

impl Default for LayoutObjective {
    /// Ölçülmemiş başlangıç profili; P5 ölçümüyle ayarlanır.
    fn default() -> Self {
        Self {
            weights: ComponentWeights::default(),
            grid: 2,
            anchor_region_ratio: 0.25,
            spacing_target_ratio: 0.05,
        }
    }
}

/// Skorlanmış yerleşmiş ürün: dünya footprint merkezi, footprint alanı ve
/// etkin rol. `placed` listesi mevcut arama durumundan türetilir; ayrı
/// durum saklanmaz (plan P2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoredItem {
    pub center: [f64; 2],
    pub area: f64,
    pub role: PlacementRole,
}

impl ScoredItem {
    /// Ürün pozundan skorlanmış öğe üretir. `auto` rolde ürün, ürün setindeki
    /// en büyük footprint'e sahipse anchor gibi puanlanır (plan P2); açıkça
    /// verilen rolde geometri çıkarımı yapılmaz.
    #[must_use]
    pub fn new(
        pose: Pose,
        footprint_centroid_local: [f64; 2],
        footprint_area_mm2: f64,
        role: PlacementRole,
        largest_footprint_area_mm2: f64,
    ) -> Self {
        Self {
            center: world_footprint_center(pose, footprint_centroid_local),
            area: footprint_area_mm2.abs(),
            role: effective_role(role, footprint_area_mm2.abs(), largest_footprint_area_mm2),
        }
    }
}

/// Dünya footprint merkezi: `pose + rotate(footprint_centroid_local, rotation)`
/// (tasarım §4.3 formülü; Pose sözleşmesi değişmez).
#[must_use]
pub fn world_footprint_center(pose: Pose, centroid_local: [f64; 2]) -> [f64; 2] {
    let (sin, cos) = pose.rotation_rad.sin_cos();
    [
        pose.x_mm + centroid_local[0] * cos - centroid_local[1] * sin,
        pose.y_mm + centroid_local[0] * sin + centroid_local[1] * cos,
    ]
}

/// Etkin rol: `auto` yalnızca ürün setindeki en büyük footprint alanına
/// sahipse anchor sayılır (eşitlik dahil).
#[must_use]
pub fn effective_role(role: PlacementRole, area: f64, largest_area: f64) -> PlacementRole {
    if role == PlacementRole::Auto && area >= largest_area {
        PlacementRole::Anchor
    } else {
        role
    }
}

/// Tam yerleşimin skoru; düşük = iyi. Boş yerleşim 0.
#[must_use]
pub fn score_layout(placed: &[ScoredItem], objective: &LayoutObjective) -> f64 {
    let w = &objective.weights;
    w.anchor * e_anchor(placed, objective)
        + w.balance * e_balance(placed)
        + w.distribution * e_distribution(placed, objective)
        + w.spacing * e_spacing(placed, objective)
        + w.orientation * e_orientation(placed)
}

/// Adayın yerleşime katkısı: tam yerleşim skoru farkı (plan P4 sıralaması
/// bunu kullanır).
#[must_use]
pub fn score_layout_delta(
    candidate: ScoredItem,
    placed: &[ScoredItem],
    objective: &LayoutObjective,
) -> f64 {
    let mut with_candidate = placed.to_vec();
    with_candidate.push(candidate);
    score_layout(&with_candidate, objective) - score_layout(placed, objective)
}

/// `E_anchor`: yalnızca anchor (etkin rol) ürünler; merkezi bölgeye normalize
/// uzaklık. Bölge içi uzaklık sıfırdır — tüm büyükleri tek noktaya çekmez.
fn e_anchor(placed: &[ScoredItem], objective: &LayoutObjective) -> f64 {
    let half = objective.anchor_region_ratio * AREA_SIZE_MM;
    let center = AREA_SIZE_MM / 2.0;
    let mut total = 0.0;
    for item in placed {
        if item.role != PlacementRole::Anchor {
            continue;
        }
        let dx = (item.center[0] - center).abs() - half;
        let dy = (item.center[1] - center).abs() - half;
        let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
        total += outside / AREA_SIZE_MM;
    }
    total
}

/// `E_balance`: alan ağırlıklı merkezin alan merkezine normalize uzaklığı.
fn e_balance(placed: &[ScoredItem]) -> f64 {
    let total_area: f64 = placed.iter().map(|i| i.area).sum();
    if total_area <= 0.0 {
        return 0.0;
    }
    let wx: f64 = placed.iter().map(|i| i.area * i.center[0]).sum();
    let wy: f64 = placed.iter().map(|i| i.area * i.center[1]).sum();
    let center = AREA_SIZE_MM / 2.0;
    ((wx / total_area - center).powi(2) + (wy / total_area - center).powi(2)).sqrt() / AREA_SIZE_MM
}

/// `E_distribution`: grid hücrelerine düşen footprint alanı dağılımının
/// dengesizliği. "Her hücrede ürün olsun" hedefi YOK (plan P2); ölçü,
/// hücre paylarının eşit paydan ortalama mutlak sapması — (Σ|sₖ − Σ/g|)/(2Σ),
/// [0,1] aralığında. Planın "maks−min" örneği yerine bu eşdeğeri seçildi:
/// maks−min, kısmi yerleşimlerde her zaman 1.0'a sabitlenip diğer bileşenleri
/// ezerdi. Ürün, alanına eşit kare olarak hücrelere bölünür (ucuz ve
/// deterministik yaklaşım).
fn e_distribution(placed: &[ScoredItem], objective: &LayoutObjective) -> f64 {
    let grid = objective.grid.max(1);
    let cell = AREA_SIZE_MM / f64::from(grid);
    let mut shares = vec![0.0_f64; (grid * grid) as usize];
    for item in placed {
        if item.area <= 0.0 {
            continue;
        }
        // Alanına eşit kare: kenar √alan, merkez dünya footprint merkezi.
        let half = item.area.sqrt() / 2.0;
        let (lo_x, hi_x) = (item.center[0] - half, item.center[0] + half);
        let (lo_y, hi_y) = (item.center[1] - half, item.center[1] + half);
        let span = |lo: f64, hi: f64| -> (i64, i64) {
            (
                ((lo / cell).floor() as i64).clamp(0, i64::from(grid) - 1),
                ((hi / cell).floor() as i64).clamp(0, i64::from(grid) - 1),
            )
        };
        let (gx0, gx1) = span(lo_x, hi_x);
        let (gy0, gy1) = span(lo_y, hi_y);
        for gy in gy0..=gy1 {
            for gx in gx0..=gx1 {
                let ox = (hi_x.min(cell * f64::from(gx as u32 + 1))
                    - lo_x.max(cell * f64::from(gx as u32)))
                .max(0.0);
                let oy = (hi_y.min(cell * f64::from(gy as u32 + 1))
                    - lo_y.max(cell * f64::from(gy as u32)))
                .max(0.0);
                shares[gy as usize * grid as usize + gx as usize] += ox * oy;
            }
        }
    }
    let total: f64 = shares.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mean = total / f64::from(grid * grid);
    let abs_dev: f64 = shares.iter().map(|s| (s - mean).abs()).sum();
    abs_dev / (2.0 * total)
}

/// `E_spacing`: hedef boşluktan yakın komşu çiftleri. Sınırsız uzaklaşmayı
/// ödüllendirmez: yalnızca hedefin altındaki yakınlık cezalandırılır.
fn e_spacing(placed: &[ScoredItem], objective: &LayoutObjective) -> f64 {
    let target = objective.spacing_target_ratio * AREA_SIZE_MM;
    let mut total = 0.0;
    for i in 0..placed.len() {
        for j in (i + 1)..placed.len() {
            let other = &placed[j];
            let dx = placed[i].center[0] - other.center[0];
            let dy = placed[i].center[1] - other.center[1];
            let dist = (dx * dx + dy * dy).sqrt();
            total += (target - dist).max(0.0) / AREA_SIZE_MM;
        }
    }
    total
}

/// `E_orientation`: yalnızca rol/fixture tanımladığında aktif olur (plan P2).
/// Modelde yönlenme tercihi verisi yok; ağırlığı başlangıçta 0'dır ve bu
/// bileşen 0 döndürür.
fn e_orientation(_placed: &[ScoredItem]) -> f64 {
    0.0
}
