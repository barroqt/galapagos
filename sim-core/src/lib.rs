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

use crate::error::SimError;
use crate::game::{Game, HawkDove};
use crate::prelude::*;
use crate::wellmixed::{WellMixed, WellMixedBuilder};
use wasm_bindgen::prelude::*;

/// A well-mixed Hawk-Dove population, driven from JavaScript.
///
/// A thin shell over [`WellMixed`]: it marshals parameters in, hands flat
/// buffers out, and turns typed errors into `JsError`. No simulation logic
/// lives here, and none may.
///
/// # Lifetime
///
/// This owns WASM-side memory that the JavaScript garbage collector does not
/// track, so the frontend must call `free()` when a run is discarded. Task
/// 2b.4 gives every wasm object an owner and a `dispose()` for exactly this.
#[wasm_bindgen]
pub struct WellMixedSim {
    inner: WellMixed,
}

#[wasm_bindgen]
impl WellMixedSim {
    /// Configures a Hawk-Dove run: resource value `v`, fight cost `c`, and a
    /// population of `population` agents starting with `initial_hawk_share`
    /// hawks.
    ///
    /// `selection_strength` and `matches_per_agent` are optional and fall
    /// back to the builder's documented defaults, so the curated entry point
    /// stays a five-argument call and expert controls are additive.
    ///
    /// `seed` crosses as a JavaScript `BigInt`, since it is a full 64-bit
    /// value. The typed wrapper in 2b.4 is the right place to do that
    /// conversion once.
    ///
    /// # Errors
    ///
    /// Returns a `JsError` describing what was rejected: a `v`/`c` pair that
    /// is not finite, a population below two, a hawk share outside `[0, 1]`,
    /// a negative or non-finite selection strength, or zero matches.
    pub fn hawk_dove(
        v: f64,
        c: f64,
        population: usize,
        initial_hawk_share: f64,
        seed: u64,
        selection_strength: Option<f64>,
        matches_per_agent: Option<usize>,
    ) -> Result<WellMixedSim, JsError> {
        configure_hawk_dove(
            v,
            c,
            population,
            initial_hawk_share,
            seed,
            selection_strength,
            matches_per_agent,
        )
        .map(|inner| Self { inner })
        .map_err(|error| JsError::new(&describe("could not configure the run", &error)))
    }

    /// Runs one generation: every agent plays its matches, then the whole
    /// population updates at once.
    ///
    /// # Errors
    ///
    /// Returns a `JsError` if a payoff total is not finite, which needs
    /// payoffs near the limits of `f64`.
    pub fn step(&mut self) -> Result<(), JsError> {
        self.inner.step().map_err(|error| {
            JsError::new(&describe("could not run a generation", &error))
        })
    }

    /// Returns how many generations have run.
    pub fn generation(&self) -> u32 {
        self.inner.generation().get()
    }

    /// Returns the number of agents.
    pub fn population(&self) -> usize {
        self.inner.population()
    }

    /// Returns the number of strategies, which is the stride of the share
    /// history.
    pub fn strategy_count(&self) -> usize {
        self.inner.game().strategy_count()
    }

    /// Returns how many generations the history holds, one more than the
    /// number of steps run.
    pub fn recorded_generations(&self) -> usize {
        self.inner.recorded_generations()
    }

    /// Returns the whole share history as a `Float64Array`, flat and
    /// generation-major: generation `g` occupies the [`strategy_count`]
    /// entries starting at `g * strategy_count`.
    ///
    /// [`strategy_count`]: WellMixedSim::strategy_count
    ///
    /// # Copy, not view
    ///
    /// This **copies** the buffer. A zero-copy `Float64Array::view` over WASM
    /// linear memory is possible and is the reason the history is stored
    /// flat, but a view is only valid until WASM memory moves, and the next
    /// `step` can do exactly that when the history outgrows its capacity.
    /// JavaScript holding such a view sees no error, only silently wrong
    /// numbers, and the bug surfaces long after the frame that caused it.
    ///
    /// The copy is therefore the default, and the cost is bounded by how it
    /// is meant to be used: call this when the run resets or a parameter
    /// changes, and use [`current_shares`] for the per-frame readout. A chart
    /// that appends one point per generation never needs the whole buffer
    /// twice.
    ///
    /// [`current_shares`]: WellMixedSim::current_shares
    ///
    /// If a long run ever makes the copy the bottleneck, the fix is a
    /// deliberate one: expose the pointer and length, build the view on the
    /// JavaScript side, and re-acquire it after every step that may have
    /// grown memory.
    pub fn share_history(&self) -> Vec<f64> {
        self.inner.share_history().to_vec()
    }

    /// Returns the current generation's shares, indexed by strategy: hawk
    /// first, then dove.
    ///
    /// One short copy per call, sized by the number of strategies rather than
    /// the length of the run, which is what makes it safe to call every
    /// frame.
    pub fn current_shares(&self) -> Vec<f64> {
        self.inner.current_shares().to_vec()
    }
}

/// Builds the simulation from the parameters JavaScript supplies.
///
/// Separate from the exported method, and returning a typed [`SimError`], so
/// that everything except the `JsError` wrap can be tested on the host:
/// `JsError` is an imported JavaScript function and panics outside wasm.
fn configure_hawk_dove(
    v: f64,
    c: f64,
    population: usize,
    initial_hawk_share: f64,
    seed: u64,
    selection_strength: Option<f64>,
    matches_per_agent: Option<usize>,
) -> Result<WellMixed, SimError> {
    let game = Game::try_from(HawkDove { v, c })?;

    // A share outside [0, 1] makes the dove share negative, which the builder
    // rejects by name, so no separate check is needed here.
    let mut builder = WellMixedBuilder::new(game, population)
        .initial_shares(vec![initial_hawk_share, 1.0 - initial_hawk_share])
        .seed(Seed::new(seed));
    if let Some(beta) = selection_strength {
        builder = builder.selection_strength(beta);
    }
    if let Some(matches) = matches_per_agent {
        builder = builder.matches_per_agent(matches);
    }
    builder.build()
}

/// Renders a typed error as the message JavaScript will see.
///
/// Kept as a plain string function so the boundary's wording is testable on
/// the host. The `Display` of these errors already names the offending value,
/// and `SimError` already repeats the cause of a wrapped `GameError`, so
/// prefixing the context is all this has to add.
fn describe(context: &str, error: &dyn std::error::Error) -> String {
    format!("{context}: {error}")
}

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

    /// The curated default: V < C, a population large enough to read and
    /// small enough to stay noisy, started at an even split.
    fn default_run() -> WellMixedSim {
        let inner = configure_hawk_dove(2.0, 4.0, 500, 0.5, 42, None, None)
            .expect("the curated default must always build");
        WellMixedSim { inner }
    }

    #[test]
    fn the_curated_default_builds_and_starts_where_it_was_asked_to() {
        let sim = default_run();

        assert_eq!(sim.population(), 500);
        assert_eq!(sim.strategy_count(), 2);
        assert_eq!(sim.generation(), 0);
        assert_eq!(sim.recorded_generations(), 1);
        assert_eq!(sim.current_shares(), [0.5, 0.5]);
    }

    #[test]
    fn the_optional_parameters_fall_back_to_the_builder_defaults() {
        let defaulted = configure_hawk_dove(2.0, 4.0, 100, 0.5, 1, None, None)
            .expect("valid configuration");
        let explicit = configure_hawk_dove(
            2.0,
            4.0,
            100,
            0.5,
            1,
            Some(WellMixedBuilder::DEFAULT_SELECTION_STRENGTH),
            Some(WellMixedBuilder::DEFAULT_MATCHES_PER_AGENT),
        )
        .expect("valid configuration");

        assert_eq!(
            defaulted.selection_strength(),
            explicit.selection_strength()
        );
        assert_eq!(
            defaulted.matches_per_agent(),
            explicit.matches_per_agent(),
            "omitting a parameter must mean the documented default"
        );
    }

    #[test]
    fn stepping_grows_the_history_by_one_generation_at_a_time() {
        let mut sim = default_run();

        for expected in 1..=5 {
            sim.step().expect("a generation runs");
            assert_eq!(sim.generation(), expected);
            assert_eq!(sim.recorded_generations(), expected as usize + 1);
        }

        let history = sim.share_history();
        assert_eq!(history.len(), sim.recorded_generations() * 2);
        assert_eq!(&history[history.len() - 2..], sim.current_shares());
    }

    #[test]
    fn the_history_crosses_as_a_flat_generation_major_buffer() {
        let mut sim = default_run();
        for _ in 0..4 {
            sim.step().expect("a generation runs");
        }
        let history = sim.share_history();

        for row in history.chunks_exact(sim.strategy_count()) {
            let sum: f64 = row.iter().sum();
            assert!((sum - 1.0).abs() < 1e-12, "row {row:?} sums to {sum}");
        }
    }

    #[test]
    fn the_history_handed_out_is_a_copy_that_the_next_step_cannot_disturb() {
        // The frontend is allowed to hold this across steps. A view into WASM
        // memory would not survive the history outgrowing its capacity.
        let mut sim = default_run();
        let snapshot = sim.share_history();

        for _ in 0..64 {
            sim.step().expect("a generation runs");
        }

        assert_eq!(snapshot, [0.5, 0.5], "the snapshot must not have moved on");
        assert!(sim.share_history().len() > snapshot.len());
    }

    #[test]
    fn a_hawk_share_outside_zero_to_one_is_rejected_by_name() {
        for bad_share in [1.5, -0.2] {
            let err = configure_hawk_dove(2.0, 4.0, 100, bad_share, 0, None, None)
                .expect_err("a share outside [0, 1] is not a proportion");
            assert!(matches!(err, SimError::InvalidShare { .. }), "{err}");
        }
    }

    #[test]
    fn invalid_parameters_are_rejected_before_a_run_exists() {
        let cases = [
            configure_hawk_dove(f64::NAN, 4.0, 100, 0.5, 0, None, None),
            configure_hawk_dove(2.0, 4.0, 1, 0.5, 0, None, None),
            configure_hawk_dove(2.0, 4.0, 100, 0.5, 0, Some(-1.0), None),
            configure_hawk_dove(2.0, 4.0, 100, 0.5, 0, None, Some(0)),
        ];

        for case in cases {
            assert!(case.is_err(), "expected a rejected configuration");
        }
    }

    #[test]
    fn a_boundary_message_keeps_both_the_context_and_the_cause() {
        // What JavaScript actually sees. The cause has to survive the wrap,
        // including through the `GameError` a `SimError` carries.
        let error = configure_hawk_dove(f64::NAN, 4.0, 100, 0.5, 0, None, None)
            .expect_err("a non-finite payoff cannot make a game");
        let message = describe("could not configure the run", &error);

        assert!(
            message.starts_with("could not configure the run: "),
            "{message}"
        );
        assert!(message.contains("not finite"), "{message}");
        assert!(message.contains(&error.to_string()), "{message}");
    }

    #[test]
    fn the_same_seed_reproduces_a_run_across_the_boundary() {
        let run = |seed| {
            let inner = configure_hawk_dove(2.0, 4.0, 300, 0.5, seed, None, None)
                .expect("valid configuration");
            let mut sim = WellMixedSim { inner };
            for _ in 0..30 {
                sim.step().expect("a generation runs");
            }
            sim.share_history()
        };

        assert_eq!(run(7), run(7));
        assert_ne!(run(7), run(8));
    }

    #[test]
    fn core_version_reports_crate_version() {
        assert_eq!(
            core_version(),
            format!("sim-core {}", env!("CARGO_PKG_VERSION"))
        );
    }
}
