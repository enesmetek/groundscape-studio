//! WASM API sınırı — plan §13.1. Byte/DTO dönüşümü, hata serileştirme ve motor
//! oturumu yaşam döngüsü burada; domain katmanı host bağımsız kalır.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::engine::Engine;
use crate::error::ImportError;
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoadProductInput {
    id: String,
    bytes: Vec<u8>,
}

/// start_placement(config, seed) → arama oturumu (plan §13.1).
#[wasm_bindgen]
pub fn start_placement(config: JsValue, seed: u64) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let config: SearchConfig =
        serde_wasm_bindgen::from_value(config).map_err(|e| JsValue::from_str(&e.to_string()))?;
    ENGINE.with(|engine| {
        engine
            .borrow_mut()
            .as_mut()
            .ok_or_else(|| JsValue::from_str("WASM_INIT_FAILED: products not loaded"))?
            .start_placement(config, seed);
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
