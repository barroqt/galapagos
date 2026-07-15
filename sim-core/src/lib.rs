//! Evolutionary game theory simulation core.
//!
//! This crate is the single source of truth for all simulation logic.
//! The web frontend (in `../web`) only renders state and forwards controls.
//!
//! Scaffold stage: only a trivial stateful counter is exported, to prove the
//! Rust → WASM → TypeScript pipeline end to end. Real modules land per issue:
//!   - game.rs      payoff matrices (Hawk-Dove, RPS, Stag Hunt, custom)
//!   - wellmixed.rs well-mixed population + analytic replicator ODE
//!   - spatial.rs   toroidal grid, local interaction, imitation updates
//!   - rng.rs       seedable RNG helpers for reproducible runs

use wasm_bindgen::prelude::*;

/// Placeholder simulation: a counter that steps. Exists solely so the
/// scaffold can verify state round-trips between Rust and the browser.
#[wasm_bindgen]
pub struct Sim {
    tick: u64,
}

#[wasm_bindgen]
impl Sim {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Sim {
        Sim { tick: 0 }
    }

    pub fn step(&mut self) -> u64 {
        self.tick += 1;
        self.tick
    }

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
}
