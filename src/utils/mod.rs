pub mod bitset;
use serde::Serialize;

pub fn to_js_str<T: Serialize>(value: &T) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|e| wasm_bindgen::JsValue::from_str(&e.to_string()))
}
pub mod dense_kernel;
<<<<<<< HEAD
pub mod static_pkt;
#[cfg(test)]
mod dense_index_proptests;
#[cfg(test)]
pub mod static_pkt_tests;
=======
>>>>>>> wreckit/zero-heap-packedkeytable-eliminate-all-latent-allocations-in-pkt-hot-paths
pub mod math;
pub mod scc;
pub mod perturbation;
