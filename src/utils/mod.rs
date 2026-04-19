pub mod bitset;
use serde::{Serialize};

pub fn to_js_str<T: Serialize>(value: &T) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
}
pub mod perturbation;
pub mod dense_kernel;
