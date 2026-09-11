//! Motor oturumu — plan §13.1. Host bağımsızdır: native testler ve WASM
//! katmanı aynı yaşam döngüsünü kullanır (load → start → step* → reset).
//!
//! Adımlama: arama, aday bütçesi dilimleriyle çalışır; Worker her adım
//! arasında mesaj döngüsüne fırsat verir (plan §13.4). Tutarlılık: aynı
//! seed/bütçe aynı sonucu verir.

use serde::Serialize;

use crate::dxf_import::import_product;
use crate::error::ImportError;
use crate::jagua_adapter::PlacementSession;
use crate::model::ProductGeometry;
use crate::scoring::LayoutObjective;
use crate::search::{PlacementResult, SearchConfig, Status, search_placement};

/// Motor durumu. Aday sayacı tüm adımları ve restartları kapsayan global
/// üst sınıra tabidir (plan §11.3).
pub struct Engine {
    products: Vec<ProductGeometry>,
    config: SearchConfig,
    objective: LayoutObjective,
    seed: u64,
    session: Option<PlacementSession>,
    candidates_used: usize,
    result: Option<PlacementResult>,
}

impl Engine {
    /// DXF byte'larını doğrulanmış ürün geometrisine çevirir. İlk hata, ürün
    /// kimliğiyle birlikte döner; yükleme oturum kurulmaz.
    pub fn load_products(inputs: &[(String, Vec<u8>)]) -> Result<Self, ImportError> {
        let mut engine = Self::empty();
        for (id, bytes) in inputs {
            engine.add_product(id, bytes)?;
        }
        Ok(engine)
    }

    pub fn empty() -> Self {
        Self {
            products: Vec::new(),
            config: SearchConfig::default(),
            objective: LayoutObjective::default(),
            seed: 0,
            session: None,
            candidates_used: 0,
            result: None,
        }
    }

    /// Tek ürün ekler; hata ürün kimliğiyle ilişkilendirilebilir.
    pub fn add_product(&mut self, id: &str, bytes: &[u8]) -> Result<(), ImportError> {
        self.products.push(import_product(id, bytes)?);
        Ok(())
    }

    /// Yüklenen ürünler; arayüz READY durumunda görselleştirir.
    pub fn products(&self) -> &[ProductGeometry] {
        &self.products
    }

    /// Skor hedefini değiştirir; sonraki aramada geçerli (plan P5 ayarı).
    pub fn set_objective(&mut self, objective: LayoutObjective) {
        self.objective = objective;
    }

    /// Arama oturumu açar; aday sayacı sıfırlanır.
    pub fn start_placement(&mut self, config: SearchConfig, seed: u64) {
        self.config = config;
        self.seed = seed;
        self.candidates_used = 0;
        self.result = None;
        self.session =
            Some(PlacementSession::new(&self.products).expect("session over validated products"));
    }

    /// Oturum temizliği.
    pub fn reset_placement(&mut self) {
        self.session = None;
        self.candidates_used = 0;
        self.result = None;
    }

    /// En fazla `max_candidates` aday harcayarak arama bir adım ilerletir.
    /// `done` olduğunda `result` kesindir; olmadığında PROGRESS verisidir.
    #[must_use]
    pub fn step_placement(&mut self, max_candidates: usize) -> StepReport {
        let Some(session) = self.session.as_mut() else {
            return StepReport {
                ran_candidates: 0,
                done: true,
                placed_count: 0,
                total_candidates: self.candidates_used,
                result: None,
            };
        };
        let remaining = self
            .config
            .max_total_candidates
            .saturating_sub(self.candidates_used);
        let step = max_candidates.min(remaining);
        if max_candidates == 0 && remaining > 0 {
            return StepReport {
                ran_candidates: 0,
                done: false,
                placed_count: self
                    .result
                    .as_ref()
                    .map_or(0, |result| result.placements.len()),
                total_candidates: self.candidates_used,
                result: None,
            };
        }

        // ponytail: bounded stateless slices; preserve search state if profiling
        // shows independent deterministic slices hurt solution quality.
        let mut config = self.config.clone();
        config.max_total_candidates = step;
        let per_item = step.div_ceil(self.products.len().max(1));
        config.local_samples_per_item = config.local_samples_per_item.min(per_item / 2);
        config.global_samples_per_item = config.global_samples_per_item.min(per_item);
        let seed = self.seed.wrapping_add(self.candidates_used as u64);
        let objective = self.objective.clone();
        let mut report = search_placement(session, &self.products, &config, &objective, seed);
        let ran = report.stats.candidates_tried.min(step);
        self.candidates_used += ran;

        if self
            .result
            .as_ref()
            .is_none_or(|best| report.placements.len() > best.placements.len())
        {
            self.result = Some(report.clone());
        }

        let done = matches!(report.status, Status::Complete | Status::InvalidInput)
            || ran == 0
            || self.candidates_used >= self.config.max_total_candidates;
        if done {
            if !matches!(report.status, Status::Complete | Status::InvalidInput) {
                report = self.result.clone().unwrap_or(report);
            }
            report.stats.candidates_tried = self.candidates_used;
            self.result = Some(report.clone());
            return self.done_step(ran, report);
        }

        StepReport {
            ran_candidates: ran,
            done: false,
            placed_count: self
                .result
                .as_ref()
                .map_or(0, |result| result.placements.len()),
            total_candidates: self.candidates_used,
            result: None,
        }
    }

    fn done_step(&self, ran: usize, result: PlacementResult) -> StepReport {
        StepReport {
            ran_candidates: ran,
            done: true,
            placed_count: result.placements.len(),
            total_candidates: self.candidates_used,
            result: Some(result),
        }
    }
}

/// Adım çıktısı: ilerleme veya kesin sonuç (plan §13.1 step_placement).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepReport {
    pub ran_candidates: usize,
    pub done: bool,
    pub placed_count: usize,
    pub total_candidates: usize,
    pub result: Option<PlacementResult>,
}
