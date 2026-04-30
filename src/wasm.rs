#![cfg(target_arch = "wasm32")]

use std::cell::Cell;
use wasm_bindgen::prelude::*;

use crate::autonomic::{
    AutonomicEvent, AutonomicFeedback, AutonomicKernel, Vision2030Kernel,
};

/// Initialize panic hook for better error messages in browser console.
#[wasm_bindgen(start)]
pub fn wasm_init() {
    console_error_panic_hook::set_once();
}

/// WASM-bound wrapper for Vision2030Kernel<4> with interior mutability via Cell.
/// All methods return scalars (u32, f32) with no heap allocation on hot paths.
#[wasm_bindgen]
pub struct WasmKernel {
    /// Wrapped kernel with WORDS=4 (256-bit bitsets)
    kernel: Cell<Vision2030Kernel<4>>,
}

#[wasm_bindgen]
impl WasmKernel {
    /// Create a new WASM kernel with default Vision2030Kernel<4>.
    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmKernel {
        WasmKernel {
            kernel: Cell::new(Vision2030Kernel::new()),
        }
    }

    /// Run a single autonomic cycle: observe(payload, source) -> propose -> execute.
    /// Returns an encoded status code: 0=success, 1=health_low, 2=conformance_low, 3=accept_failed.
    ///
    /// # Arguments
    /// * `payload` - The event payload (e.g., "Start", "Normal", "violation")
    /// * `source` - The event source identifier
    ///
    /// # Returns
    /// Status code (u32): 0 = cycle executed, >0 = early exit with reason
    #[wasm_bindgen]
    pub fn cycle(&self, payload: String, source: String) -> u32 {
        let event = AutonomicEvent {
            source,
            payload,
            timestamp: std::time::SystemTime::now(),
        };

        // Use interior mutability to borrow_mut inside &self
        let mut k = self.kernel.take();
        let results = k.run_cycle(event);
        self.kernel.set(k);

        if results.is_empty() {
            2 // Early exit: conformance or health too low
        } else {
            0 // Success: at least one action executed
        }
    }

    /// Get current process health score [0.0, 1.0].
    /// Returns as u32 * 10000 to preserve 4 decimal places (e.g., 8500 = 0.85).
    #[wasm_bindgen]
    pub fn health(&self) -> f32 {
        let k = self.kernel.take();
        let h = k.infer().process_health;
        self.kernel.set(k);
        h
    }

    /// Get current conformance score [0.0, 1.0].
    /// Returns as u32 * 10000 to preserve 4 decimal places (e.g., 7500 = 0.75).
    #[wasm_bindgen]
    pub fn conformance(&self) -> f32 {
        let k = self.kernel.take();
        let c = k.infer().conformance_score;
        self.kernel.set(k);
        c
    }

    /// Apply feedback to adapt kernel state.
    /// Positive reward [0.0, 1.0] reinforces current behavior.
    /// Negative reward [-1.0, 0.0] penalizes and triggers repair.
    #[wasm_bindgen]
    pub fn feedback(&self, reward: f32) {
        let clamped_reward = reward.clamp(-1.0, 1.0);
        let feedback = AutonomicFeedback {
            reward: clamped_reward,
            human_override: clamped_reward < 0.0,
            side_effects: vec![],
        };

        let mut k = self.kernel.take();
        k.adapt(feedback);
        self.kernel.set(k);
    }

    /// Recommended batch size hint for processing events in parallel.
    /// Returns batch size as u32 (typically 32-256).
    #[wasm_bindgen]
    pub fn batch_size_hint(&self) -> u32 {
        let k = self.kernel.take();
        let config = &k.config;
        let hint = config.wasm.batch_size;
        self.kernel.set(k);
        hint as u32
    }

    /// Recommended maximum pages for event log buffering.
    /// Returns page count as u32 (typically 8-32 pages, 4KB each = 32-128KB buffer).
    #[wasm_bindgen]
    pub fn max_pages_hint(&self) -> u32 {
        let k = self.kernel.take();
        let hint = k.config.wasm.max_pages;
        self.kernel.set(k);
        hint as u32
    }
}

impl Default for WasmKernel {
    fn default() -> Self {
        Self::new()
    }
}
