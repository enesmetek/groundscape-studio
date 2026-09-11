//! WASM API sınırı — plan §13.1. Byte/DTO dönüşümü, hata serileştirme ve motor
//! oturumu yaşam döngüsü burada; domain katmanı host bağımsız kalır.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::engine::Engine;
use crate::error::ImportError;
use crate::model::PlacementRole;
use crate::scoring::LayoutObjective;
use crate::search::SearchConfig;

thread_local! {
    static ENGINE: std::cell::RefCell<Option<Engine>> = const { std::cell::RefCell::new(None) };
}

/// Hata DTO'su: plan §13.1 — serbest metin değil, kod/faz/ürün/açıklama.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiError {
    code: &'static str,
    phase: &'static str,
    #[serde(rename = "productId")]
    product_id: Option<String>,
    description: String,
}

fn import_error(product_id: &str, error: &ImportError) -> JsValue {
    serde_wasm_bindgen::to_value(&ApiError {
        code: error.code(),
        phase: "import",
        product_id: Some(product_id.to_owned()),
        description: error.to_string(),
    })
    .unwrap_or_else(|_| JsValue::from_str(&error.to_string()))
}

/// load_products(inputs) → geometriler + alan bilgisi (plan §13.1).
/// READY yükü: alan boyutu, canonical geometriler, safety alanları.
#[wasm_bindgen]
pub fn load_products(inputs: JsValue) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();
    let inputs: Vec<LoadProductInput> =
        serde_wasm_bindgen::from_value(inputs).map_err(|e| JsValue::from_str(&e.to_string()))?;

    let mut engine = Engine::empty();
    for input in &inputs {
        engine
            .add_product(&input.id, &input.bytes)
            .map_err(|error| import_error(&input.id, &error))?;
        // Rol/etiket override: import DXF'ten taşımadığı için burada uygulanır.
        if input.placement_role.is_some() || input.tags.is_some() || input.age_group.is_some() {
            engine.override_product_metadata(
                &input.id,
                input.placement_role.unwrap_or_default(),
                input.tags.clone().unwrap_or_default(),
                input.age_group.clone(),
            );
        }
    }

    let ready = ReadyPayload {
        area_mm: crate::area::AREA_SIZE_MM,
        products: engine
            .products()
            .iter()
            .map(|product| ProductPayload {
                id: product.id.clone(),
                footprint_polygons: product.footprint_polygons.clone(),
                safety_zone: product.safety_zone.clone(),
                safety_area_mm2: product.safety_area_mm2,
                footprint_area_mm2: product.footprint_area_mm2,
                footprint_centroid_local: product.footprint_centroid_local,
                placement_role: product.placement_role,
            })
            .collect(),
    };
    // Engine kalıcı oturumda tutulur; step_placement aynı oturumu kullanır.
    ENGINE.with(|e| e.borrow_mut().replace(engine));
    serde_wasm_bindgen::to_value(&ready).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadyPayload {
    area_mm: f64,
    products: Vec<ProductPayload>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductPayload {
    id: String,
    footprint_polygons: Vec<crate::geometry::Polygon>,
    safety_zone: crate::geometry::Polygon,
    safety_area_mm2: f64,
    footprint_area_mm2: f64,
    footprint_centroid_local: [f64; 2],
    placement_role: crate::model::PlacementRole,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadProductInput {
    id: String,
    bytes: Vec<u8>,
    /// DXF rol taşımadığı için opsiyonel; yoksa `auto` (MVP-2 plan P5.5).
    placement_role: Option<PlacementRole>,
    tags: Option<Vec<String>>,
    age_group: Option<String>,
}

/// start_placement(config, seed, objective) → arama oturumu (plan §13.1).
/// `objective` opsiyonel: undefined/null ise `LayoutObjective::default()`.
#[wasm_bindgen]
pub fn start_placement(config: JsValue, seed: u64, objective: JsValue) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let config: SearchConfig =
        serde_wasm_bindgen::from_value(config).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let objective: LayoutObjective = if objective.is_undefined() || objective.is_null() {
        LayoutObjective::default()
    } else {
        serde_wasm_bindgen::from_value(objective).map_err(|e| JsValue::from_str(&e.to_string()))?
    };
    ENGINE.with(|engine| {
        engine
            .borrow_mut()
            .as_mut()
            .ok_or_else(|| JsValue::from_str("WASM_INIT_FAILED: products not loaded"))?
            .start_placement(config, seed);
        // ponytail: set_objective yalnızca bu kanaldan beslenir; UI kendi
        // ağırlık setini gönderene kadar default kullanılır.
        engine
            .borrow_mut()
            .as_mut()
            .expect("engine checked above")
            .set_objective(objective);
        Ok(())
    })
}

/// step_placement(maxCandidates) → ilerleme veya kesin sonuç (plan §13.1).
/// Worker her adım arasında mesaj döngüsüne fırsat verir.
#[wasm_bindgen]
pub fn step_placement(max_candidates: usize) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();
    let report = ENGINE.with(|engine| {
        let report = engine
            .borrow_mut()
            .as_mut()
            .ok_or_else(|| JsValue::from_str("WASM_INIT_FAILED: products not loaded"))?
            .step_placement(max_candidates);
        serde_wasm_bindgen::to_value(&report).map_err(|e| JsValue::from_str(&e.to_string()))
    })?;
    Ok(report)
}

/// reset_placement() → oturum temizliği (plan §13.1).
#[wasm_bindgen]
pub fn reset_placement() {
    console_error_panic_hook::set_once();
    ENGINE.with(|engine| {
        if let Some(engine) = engine.borrow_mut().as_mut() {
            engine.reset_placement();
        }
    });
}
