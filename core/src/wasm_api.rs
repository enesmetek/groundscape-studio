use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_spike_wasm() -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();
    let result = crate::run_spike().map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_wasm_bindgen::to_value(&result).map_err(|error| JsValue::from_str(&error.to_string()))
}
