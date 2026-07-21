//! Evolutionary game theory simulation core.
//!
//! This crate is the single source of truth for all simulation logic. The web
//! frontend (in `../web`) only renders state and forwards controls.
//!
//! This file is the **wasm boundary and nothing else**: it declares the module
//! tree and holds the `#[wasm_bindgen]` exports, which stay a thin shell over
//! the pure domain logic in the modules below. State crosses to JS as flat
//! buffers, never as nested objects, and typed errors become `JsError` here
//! rather than anywhere deeper.

#![warn(missing_docs)]

pub mod error;
pub mod game;
pub mod prelude;
pub mod rng;
pub mod types;
pub mod wellmixed;

use wasm_bindgen::prelude::*;

/// Placeholder simulation: a counter that steps. Exists solely so the
/// scaffold can verify state round-trips between Rust and the browser.
///
/// `web/` still imports this; it is deleted in Task 3a.0, once Task 2b.3 has
/// removed the last frontend reference.
#[wasm_bindgen]
pub struct Sim {
    tick: u64,
}

#[wasm_bindgen]
impl Sim {
    /// Creates a counter sitting at tick 0.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Sim {
        Sim { tick: 0 }
    }

    /// Advances the counter and returns the new tick.
    pub fn step(&mut self) -> u64 {
        self.tick += 1;
        self.tick
    }

    /// Returns the current tick without advancing it.
    pub fn tick(&self) -> u64 {
        self.tick
    }
}

impl Default for Sim {
    fn default() -> Self {
        Self::new()
    }
}

/// Version string surfaced in the UI footer to confirm which core is loaded.
#[wasm_bindgen]
pub fn core_version() -> String {
    format!("sim-core {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_steps() {
        let mut sim = Sim::new();
        assert_eq!(sim.step(), 1);
        assert_eq!(sim.step(), 2);
        assert_eq!(sim.tick(), 2);
    }

    #[test]
    fn core_version_reports_crate_version() {
        assert_eq!(
            core_version(),
            format!("sim-core {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
