//! Replicator dynamics: the deterministic limit the agent-based run wanders
//! around.
//!
//! The replicator equation says a strategy grows at the rate by which it beats
//! the population average,
//!
//! ```text
//! x_i' = x_i * (f_i(x) - phi(x)),   f(x) = A x,   phi(x) = x . f(x)
//! ```
//!
//! For two strategies that is the familiar `x' = x(1-x)(f_H - f_D)`, but it is
//! written here for `n` from the start: Issue 4a puts Rock-Paper-Scissors
//! through the same integrator, and a two-strategy special case would have to
//! be thrown away to get there.
//!
//! Nothing in this module is stochastic. There is no [`Seed`], no `Rng`, and
//! no hidden state - the same configuration always produces the same
//! trajectory. That is the point: it is the curve the noisy, finite run is
//! measured against.

use crate::game::Game;
use crate::prelude::*;
use crate::shares::Shares;

/// Configuration for a [`Replicator`] trajectory, validated in [`build`].
///
/// [`build`]: ReplicatorBuilder::build
///
/// The game has no sensible default and is given up front; the initial state
/// carries one, so the shortest configuration that means anything is also the
/// shortest to write:
///
/// ```
/// use sim_core::game::{Game, HawkDove};
/// use sim_core::replicator::ReplicatorBuilder;
///
/// let game = Game::try_from(HawkDove { v: 2.0, c: 4.0 })?;
/// let mut ode = ReplicatorBuilder::new(game).build()?;
/// ode.run(1_000, 0.01)?;
///
/// // V/C = 0.5, reached from the even split it started at.
/// assert!((ode.current_shares()[0] - 0.5).abs() < 1e-9);
/// # Ok::<(), sim_core::error::SimError>(())
/// ```
///
/// The time step is not configured here. It belongs to [`Replicator::step`]
/// and [`Replicator::run`] because it is a property of how far the caller
/// wants to advance, not of the population being advanced, and because the UI
/// changes it without rebuilding the run.
#[derive(Debug, Clone)]
pub struct ReplicatorBuilder {
    game: Game,
    initial_shares: Option<Vec<f64>>,
}

impl ReplicatorBuilder {
    /// Starts a configuration for a population playing `game`.
    ///
    /// The argument is not validated here. `build` is the single place that
    /// can fail, so a half-built configuration never has to be considered.
    pub fn new(game: Game) -> Self {
        Self {
            game,
            initial_shares: None,
        }
    }

    /// Sets the share of the population starting on each strategy, indexed by
    /// strategy. Defaults to an even split.
    ///
    /// The shares must be proportions summing to 1; a strategy may start at
    /// zero, and the equation keeps it there.
    pub fn initial_shares(mut self, shares: Vec<f64>) -> Self {
        self.initial_shares = Some(shares);
        self
    }

    /// Validates the configuration and allocates every buffer a step writes
    /// into, so integrating is allocation-free.
    ///
    /// # Errors
    ///
    /// Returns [`SimError`] if the shares do not describe the game's
    /// strategies as proportions summing to 1.
    pub fn build(self) -> Result<Replicator, SimError> {
        let strategy_count = self.game.strategy_count();
        let shares = match self.initial_shares {
            Some(values) => Shares::checked(values, strategy_count)?,
            None => Shares::uniform(strategy_count),
        };

        let state = Vec::from(shares);
        // Step 0 is the initial condition, recorded before anything runs so
        // this trajectory and the agent-based run it overlays start from the
        // same point rather than one row apart.
        let history = state.clone();

        Ok(Replicator {
            game: self.game,
            next: state.clone(),
            probe: state.clone(),
            fitness: vec![0.0; strategy_count],
            stages: std::array::from_fn(|_| vec![0.0; strategy_count]),
            state,
            history,
            generation: Generation::ZERO,
        })
    }
}

/// A replicator trajectory: an infinite well-mixed population, integrated.
///
/// This is the same game and the same equilibrium as [`WellMixed`], without
/// the finite-population noise - which is exactly the contrast Issue 2a
/// exists to show. Built through [`ReplicatorBuilder`], the only place a
/// configuration can be rejected.
///
/// [`WellMixed`]: crate::wellmixed::WellMixed
#[derive(Debug)]
pub struct Replicator {
    game: Game,
    /// Current state `x`, one share per strategy. Always on the simplex; see
    /// [`Replicator::step`] for what keeps it there.
    state: Vec<f64>,
    /// Target of the step in progress. A step reads `state` and writes here,
    /// and the two are swapped only once the result is known to be on the
    /// simplex, so a rejected step leaves the trajectory untouched.
    next: Vec<f64>,
    /// The point an RK4 stage is evaluated at, `state + factor * k`.
    probe: Vec<f64>,
    /// Fitness `A x` at whichever point is currently being evaluated.
    fitness: Vec<f64>,
    /// The four RK4 stage derivatives, `k1` to `k4`.
    stages: [Vec<f64>; 4],
    /// Shares per step, generation-major and flat. See
    /// [`Replicator::share_history`] for the layout and why it is one buffer.
    history: Vec<f64>,
    generation: Generation,
}

impl Replicator {
    /// How far off the simplex a state may drift before the step that
    /// produced it is rejected.
    ///
    /// # Why repair the small case at all
    ///
    /// The sum is conserved *exactly* by this scheme, not merely to fourth
    /// order: every stage derivative sums to zero when the point it is
    /// evaluated at sums to one, so each probe and the final combination do
    /// too. What is left is floating-point rounding, which accumulates over a
    /// long run and would otherwise show up as shares that visibly stop adding
    /// to 1. Clamping a share of `-1e-17` to zero and rescaling by a sum of
    /// `1 + 3e-16` removes that drift and changes nothing a caller can see.
    ///
    /// # Why not repair the large case
    ///
    /// Renormalising unconditionally would be the same three lines of code and
    /// would hide the failure this constant exists to expose. A time step too
    /// large for the game sends a share below zero or above one; rescaled, the
    /// result still sums to 1, still looks like a distribution, and is not a
    /// solution of the equation. The caller would see a smooth wrong curve
    /// with nothing to indicate it. So anything past this bound is a typed
    /// error naming the step that has to shrink.
    ///
    /// The bound is far above the rounding it forgives (a few multiples of
    /// `f64::EPSILON` per step) and far below any real excursion, which grows
    /// with `dt` rather than creeping.
    pub const SIMPLEX_TOLERANCE: f64 = 1e-9;

    /// Returns the game being played.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// Returns how many steps have been integrated.
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// Returns the current state, indexed by strategy: for Hawk-Dove, hawk
    /// first and then dove.
    ///
    /// Always `strategy_count` entries, and always equal to the last row of
    /// [`share_history`]: step 0 is recorded at build time, so there is never
    /// a moment with no current state.
    ///
    /// [`share_history`]: Replicator::share_history
    pub fn current_shares(&self) -> &[f64] {
        &self.state
    }

    /// Returns the state at every step so far, flat and generation-major:
    /// step `g`'s shares are the `strategy_count` entries starting at
    /// `g * strategy_count`.
    ///
    /// This is the layout [`WellMixed::share_history`] uses, deliberately and
    /// exactly: the UI overlays the analytic curve on the simulated one by
    /// reading two `Float64Array`s with the same stride, and neither is
    /// reshaped on the way.
    ///
    /// It always holds at least step 0, which is recorded at build time, and
    /// it grows by one row per [`step`].
    ///
    /// [`WellMixed::share_history`]: crate::wellmixed::WellMixed::share_history
    /// [`step`]: Replicator::step
    pub fn share_history(&self) -> &[f64] {
        &self.history
    }

    /// Returns how many steps are recorded, which is one more than the number
    /// integrated.
    pub fn recorded_generations(&self) -> usize {
        // Exact: the history only ever grows by whole rows.
        self.history.len() / self.game.strategy_count()
    }

    /// Returns one step's shares, or `None` if that step has not run yet.
    pub fn shares_at(&self, generation: Generation) -> Option<&[f64]> {
        let strategy_count = self.game.strategy_count();
        let base = generation.index().checked_mul(strategy_count)?;
        self.history.get(base..base.checked_add(strategy_count)?)
    }

    /// Advances the trajectory by `dt` using classical fourth-order
    /// Runge-Kutta, and records the new state.
    ///
    /// Four derivative evaluations per step rather than one, in exchange for
    /// an error per step of order `dt^5` instead of `dt^2`. That trade is what
    /// lets the UI take steps large enough to cover a whole run at interactive
    /// speed and still draw a curve whose shape is the equation's rather than
    /// the integrator's.
    ///
    /// The result is checked against the simplex before it is committed, and
    /// rounding-level drift is removed; see [`SIMPLEX_TOLERANCE`] for the
    /// reasoning, which is the substantive design decision in this module.
    ///
    /// [`SIMPLEX_TOLERANCE`]: Replicator::SIMPLEX_TOLERANCE
    ///
    /// # Errors
    ///
    /// Returns [`SimError::InvalidTimeStep`] if `dt` is not finite and
    /// positive, and [`SimError::LeftTheSimplex`] if the step was too large
    /// for this game. In either case the trajectory is left exactly as it was,
    /// so the caller can retry with a smaller step.
    pub fn step(&mut self, dt: f64) -> Result<(), SimError> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(SimError::InvalidTimeStep { value: dt });
        }

        self.integrate(dt)?;
        settle_onto_simplex(&mut self.next, dt)?;

        // O(1), and it is what makes the commit atomic: until this line the
        // trajectory still holds the state the step started from.
        std::mem::swap(&mut self.state, &mut self.next);
        self.generation = self.generation.next();
        self.history.extend_from_slice(&self.state);
        Ok(())
    }

    /// Advances the trajectory by `steps` steps of `dt`.
    ///
    /// The history is grown to its final size once here rather than a row at a
    /// time, since the length of the run is known before it starts.
    ///
    /// # Errors
    ///
    /// Propagates the first failing [`step`], leaving the trajectory at the
    /// last state that was on the simplex.
    ///
    /// [`step`]: Replicator::step
    pub fn run(&mut self, steps: usize, dt: f64) -> Result<(), SimError> {
        // Saturating rather than wrapping: a request this large cannot be
        // served either way, and an under-reserved buffer merely grows the way
        // a `Vec` normally does instead of reserving nonsense.
        self.history
            .reserve(steps.saturating_mul(self.game.strategy_count()));

        for _ in 0..steps {
            self.step(dt)?;
        }
        Ok(())
    }

    /// Writes one RK4 step from `state` into `next`, touching no allocation.
    ///
    /// # Errors
    ///
    /// Propagates the failure of any stage evaluation.
    fn integrate(&mut self, dt: f64) -> Result<(), SimError> {
        // Destructured so the borrow checker sees the game, the state, the
        // stage buffers and the scratch space as disjoint borrows rather than
        // one borrow of `self`.
        let Self {
            game,
            state,
            next,
            probe,
            fitness,
            stages,
            ..
        } = self;
        let [k1, k2, k3, k4] = stages;
        let half = dt / 2.0;

        derivative(game, state, fitness, k1)?;
        offset(probe, state, k1, half);
        derivative(game, probe, fitness, k2)?;
        offset(probe, state, k2, half);
        derivative(game, probe, fitness, k3)?;
        offset(probe, state, k3, dt);
        derivative(game, probe, fitness, k4)?;

        let sixth = dt / 6.0;
        let stage_sum = k1.iter().zip(k2.iter()).zip(k3.iter().zip(k4.iter()));
        for (slot, (&x, ((&a, &b), (&c, &d)))) in
            next.iter_mut().zip(state.iter().zip(stage_sum))
        {
            *slot = x + sixth * (a + 2.0 * (b + c) + d);
        }
        Ok(())
    }
}

/// Writes the expected payoff of each strategy against the population `x` into
/// `out`, and returns the population's mean fitness `x . f`.
///
/// This is `f(x) = A x`: the exact quantity the agent-based matching pass
/// samples by drawing opponents. One pass over the matrix rather than one
/// lookup per pair, and the mean is accumulated in the same pass rather than
/// in a second one over `out`.
///
/// # Errors
///
/// Returns [`SimError::ShareCountMismatch`] if either buffer disagrees with
/// the game, and [`SimError::UnknownStrategy`] if the matrix is missing a row
/// the state has a share for. The builder sizes every buffer from the game, so
/// neither is reachable through the public API; they are reported rather than
/// ignored because a silently truncated fitness vector would bias a
/// trajectory in a way no shape check would catch.
fn fitness(game: &Game, x: &[f64], out: &mut [f64]) -> Result<f64, SimError> {
    let strategy_count = game.strategy_count();
    for found in [x.len(), out.len()] {
        if found != strategy_count {
            return Err(SimError::ShareCountMismatch {
                found,
                expected: strategy_count,
            });
        }
    }

    let mut mean = 0.0;
    for (strategy, (slot, &share)) in out.iter_mut().zip(x.iter()).enumerate() {
        // In range: `strategy_count` is at most `Game::MAX_STRATEGIES` = 256.
        let id = StrategyId::new(strategy as u8);
        let row = game.row(id).ok_or(SimError::UnknownStrategy {
            strategy,
            strategy_count,
        })?;

        let payoff: f64 = row
            .iter()
            .zip(x.iter())
            .map(|(entry, opponent_share)| entry * opponent_share)
            .sum();
        *slot = payoff;
        mean += share * payoff;
    }
    Ok(mean)
}

/// Writes the replicator derivative at `x` into `out`, using `scratch` for the
/// fitness vector.
///
/// # Errors
///
/// Propagates the failure of the fitness evaluation.
fn derivative(
    game: &Game,
    x: &[f64],
    scratch: &mut [f64],
    out: &mut [f64],
) -> Result<(), SimError> {
    let mean = fitness(game, x, scratch)?;

    for (slot, (&share, &payoff)) in out.iter_mut().zip(x.iter().zip(scratch.iter())) {
        // The `x_i` factor is what makes zero an absorbing share: the
        // equation describes strategies that reproduce, and nothing
        // reproduces from nothing.
        *slot = share * (payoff - mean);
    }
    Ok(())
}

/// Writes `base + factor * direction` into `dest`.
///
/// The point an RK4 stage is evaluated at, written into a buffer the caller
/// owns so a step never allocates.
fn offset(dest: &mut [f64], base: &[f64], direction: &[f64], factor: f64) {
    for (slot, (&origin, &step)) in dest.iter_mut().zip(base.iter().zip(direction.iter()))
    {
        *slot = origin + factor * step;
    }
}

/// Removes rounding-level drift off the simplex, and rejects anything larger.
///
/// See [`Replicator::SIMPLEX_TOLERANCE`] for why the two cases are treated
/// differently rather than both being renormalised away.
///
/// # Errors
///
/// Returns [`SimError::LeftTheSimplex`] if a share is outside `[0, 1]` by more
/// than the tolerance, or is not finite, and
/// [`SimError::SharesNotNormalised`] if the shares no longer add up to a whole
/// population. `state` may have been partly clamped when either is returned,
/// which is why the caller checks the buffer it is about to commit rather than
/// the one it would have to roll back.
fn settle_onto_simplex(state: &mut [f64], time_step: f64) -> Result<(), SimError> {
    let tolerance = Replicator::SIMPLEX_TOLERANCE;

    let mut sum = 0.0;
    for (strategy, share) in state.iter_mut().enumerate() {
        if !share.is_finite() || *share < -tolerance || *share > 1.0 + tolerance {
            return Err(SimError::LeftTheSimplex {
                strategy,
                value: *share,
                time_step,
            });
        }
        *share = share.clamp(0.0, 1.0);
        sum += *share;
    }

    if (sum - 1.0).abs() > tolerance {
        return Err(SimError::SharesNotNormalised { sum, tolerance });
    }
    // `sum` is within a rounding error of 1, so this cannot divide by zero and
    // cannot move a share far enough to matter - it only stops the drift from
    // accumulating over a long run.
    for share in state.iter_mut() {
        *share /= sum;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::HawkDove;

    fn hawk_dove(v: f64, c: f64) -> Game {
        Game::try_from(HawkDove { v, c }).expect("valid game")
    }

    fn build(v: f64, c: f64, initial_hawk_share: f64) -> Replicator {
        ReplicatorBuilder::new(hawk_dove(v, c))
            .initial_shares(vec![initial_hawk_share, 1.0 - initial_hawk_share])
            .build()
            .expect("valid configuration")
    }

    /// A game in which each strategy earns the same whoever it meets, so
    /// fitness is constant and the two-strategy equation reduces to the
    /// logistic one - the only case with a closed form to measure against.
    fn constant_fitness_game(advantage: f64) -> Game {
        Game::try_from(vec![vec![advantage, advantage], vec![0.0, 0.0]])
            .expect("valid game")
    }

    /// The exact solution of `x' = x(1-x)r` from `x0` after time `t`.
    fn logistic(x0: f64, r: f64, t: f64) -> f64 {
        let growth = (r * t).exp();
        x0 * growth / (1.0 - x0 + x0 * growth)
    }

    #[test]
    fn fitness_is_the_matrix_times_the_state() {
        // Hand-computed against V=2, C=4 at x = (0.25, 0.75):
        // f_H = 0.25 * -1 + 0.75 * 2 = 1.25, f_D = 0.25 * 0 + 0.75 * 1 = 0.75.
        let game = hawk_dove(2.0, 4.0);
        let mut out = [0.0; 2];

        let mean = fitness(&game, &[0.25, 0.75], &mut out).expect("sized buffers");

        assert_eq!(out, [1.25, 0.75]);
        assert_eq!(mean, 0.25 * 1.25 + 0.75 * 0.75);
    }

    #[test]
    fn the_two_strategies_earn_the_same_at_the_equilibrium_share() {
        // This is *why* V/C is the equilibrium: at that share neither strategy
        // has any advantage to grow on. Checked for two (V, C) pairs so it
        // pins the formula rather than the number 0.5.
        for (v, c) in [(2.0, 4.0), (1.0, 3.0)] {
            let game = hawk_dove(v, c);
            let equilibrium = v / c;
            let mut out = [0.0; 2];

            let mean = fitness(&game, &[equilibrium, 1.0 - equilibrium], &mut out)
                .expect("sized buffers");

            assert!(
                (out[0] - out[1]).abs() < 1e-15,
                "V={v}, C={c}: hawk earns {}, dove earns {}",
                out[0],
                out[1]
            );
            assert!((mean - out[0]).abs() < 1e-15, "the mean is that same value");
        }
    }

    #[test]
    fn a_buffer_that_does_not_match_the_game_is_rejected() {
        let game = hawk_dove(2.0, 4.0);
        let mut out = [0.0; 2];

        assert!(matches!(
            fitness(&game, &[1.0], &mut out),
            Err(SimError::ShareCountMismatch {
                found: 1,
                expected: 2
            })
        ));
        assert!(matches!(
            fitness(&game, &[0.5, 0.5], &mut [0.0; 3]),
            Err(SimError::ShareCountMismatch {
                found: 3,
                expected: 2
            })
        ));
    }

    #[test]
    fn the_derivative_conserves_the_population() {
        // Shares are proportions of one population, so whatever the dynamics
        // do, the growth rates have to cancel. This is what lets the sum be
        // repaired by rounding rather than renormalised by fiat.
        let game = Game::try_from(vec![
            vec![0.0, 1.0, -1.0],
            vec![-1.0, 0.0, 1.0],
            vec![1.0, -1.0, 0.0],
        ])
        .expect("valid game");
        let mut scratch = [0.0; 3];
        let mut out = [0.0; 3];

        for x in [[1.0 / 3.0; 3], [0.2, 0.3, 0.5], [0.9, 0.1, 0.0]] {
            derivative(&game, &x, &mut scratch, &mut out).expect("sized buffers");
            let total: f64 = out.iter().sum();
            assert!(total.abs() < 1e-15, "growth rates sum to {total} at {x:?}");
        }
    }

    #[test]
    fn a_strategy_at_zero_has_no_growth_rate_at_all() {
        // Exactly zero, not nearly: the integrator must not be able to
        // resurrect an extinct strategy out of rounding.
        let game = hawk_dove(2.0, 4.0);
        let mut scratch = [0.0; 2];
        let mut out = [0.0; 2];

        derivative(&game, &[0.0, 1.0], &mut scratch, &mut out).expect("sized buffers");

        assert_eq!(out, [0.0, 0.0]);
    }

    #[test]
    fn halving_the_step_cuts_the_error_by_roughly_sixteen() {
        // Fourth order means the error per unit time scales with dt^4, so
        // halving dt should divide it by about 16. Measured against the
        // logistic closed form, since a reference produced by the same
        // integrator would only be measuring it against itself.
        let (x0, rate, duration) = (0.1, 1.0, 1.0);
        let exact = logistic(x0, rate, duration);

        let error_at = |dt: f64| {
            let steps = (duration / dt).round() as usize;
            let mut ode = ReplicatorBuilder::new(constant_fitness_game(rate))
                .initial_shares(vec![x0, 1.0 - x0])
                .build()
                .expect("valid configuration");
            ode.run(steps, dt).expect("the trajectory stays bounded");
            (ode.current_shares()[0] - exact).abs()
        };

        let coarse = error_at(0.2);
        let fine = error_at(0.1);
        let ratio = coarse / fine;

        assert!(
            coarse > 0.0 && fine > 0.0,
            "the comparison needs real error"
        );
        assert!(
            (10.0..24.0).contains(&ratio),
            "expected roughly a 16x improvement, got {ratio} ({coarse} then {fine})"
        );
    }

    #[test]
    fn the_integrator_is_far_more_accurate_than_a_single_euler_step_would_be() {
        // Guards the four stages themselves: an implementation that used only
        // k1 would still converge to the right equilibrium and still conserve
        // the sum, and would sit around 1e-3 here rather than 1e-8.
        let (x0, rate, dt) = (0.1, 1.0, 0.1);
        let mut ode = ReplicatorBuilder::new(constant_fitness_game(rate))
            .initial_shares(vec![x0, 1.0 - x0])
            .build()
            .expect("valid configuration");

        ode.run(10, dt).expect("the trajectory stays bounded");

        let error = (ode.current_shares()[0] - logistic(x0, rate, 1.0)).abs();
        assert!(
            error < 1e-6,
            "fourth-order error should be tiny, got {error}"
        );
    }

    #[test]
    fn a_long_run_from_the_edge_of_the_simplex_stays_on_it() {
        // 100_000 steps starting one part in a million from the boundary: the
        // regime where a share can be pushed negative and where the sum has
        // the most rounding to accumulate.
        let mut ode = build(2.0, 4.0, 1e-6);

        ode.run(100_000, 0.01)
            .expect("the trajectory stays on the simplex");

        for (step, shares) in ode.share_history().chunks_exact(2).enumerate() {
            let sum: f64 = shares.iter().sum();
            assert!(
                shares.iter().all(|&s| (0.0..=1.0).contains(&s)),
                "step {step} left the simplex: {shares:?}"
            );
            assert!((sum - 1.0).abs() < 1e-12, "step {step} sums to {sum}");
        }
        assert!((ode.current_shares()[0] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn a_step_too_large_for_the_game_is_reported_rather_than_renormalised() {
        // The failure this module's design decision exists to expose. With a
        // step of 5 time units the cubic overshoots hard; renormalising would
        // hand back something that still sums to 1 and is not a solution.
        let mut ode = build(2.0, 40.0, 0.9);

        let err = ode
            .step(5.0)
            .expect_err("a wildly oversized step cannot stay on the simplex");

        assert!(
            matches!(err, SimError::LeftTheSimplex { time_step, .. } if time_step == 5.0),
            "{err}"
        );
    }

    #[test]
    fn a_rejected_step_leaves_the_trajectory_exactly_where_it_was() {
        let mut ode = build(2.0, 40.0, 0.9);
        let before = ode.share_history().to_vec();

        ode.step(5.0).expect_err("an oversized step is rejected");

        assert_eq!(ode.share_history(), before);
        assert_eq!(ode.current_shares(), &before[..2]);
        assert_eq!(ode.generation(), Generation::ZERO);

        // And the run is still usable at a step that fits.
        ode.step(0.01).expect("a sane step still works");
        assert_eq!(ode.generation(), Generation::ZERO.next());
    }

    #[test]
    fn a_step_that_is_not_a_positive_length_of_time_is_rejected() {
        let mut ode = build(2.0, 4.0, 0.5);

        for bad in [0.0, -0.01, f64::NAN, f64::INFINITY] {
            let err = ode.step(bad).expect_err("a step must advance time");
            assert!(matches!(err, SimError::InvalidTimeStep { .. }), "{err}");
        }
        assert_eq!(ode.recorded_generations(), 1, "nothing was recorded");
    }

    #[test]
    fn rounding_level_drift_is_repaired_rather_than_reported() {
        // The small case, exercised directly: a share a hundredth of the
        // tolerance below zero is rounding, and the caller should never hear
        // about it.
        let mut state = [-1e-11, 1.0];

        settle_onto_simplex(&mut state, 0.01).expect("rounding is not a failure");

        assert_eq!(state[0], 0.0, "a negative rounding artefact is clamped");
        assert!((state.iter().sum::<f64>() - 1.0).abs() < 1e-15);
    }

    #[test]
    fn every_scratch_buffer_is_sized_once_at_build_time() {
        // Stepping must never allocate, which it can only guarantee if the
        // buffers it writes into already have their final length here.
        let ode = build(2.0, 4.0, 0.5);

        assert_eq!(ode.next.len(), 2);
        assert_eq!(ode.probe.len(), 2);
        assert_eq!(ode.fitness.len(), 2);
        assert!(ode.stages.iter().all(|stage| stage.len() == 2));
    }

    #[test]
    fn integrating_reuses_its_buffers_instead_of_reallocating() {
        let mut ode = build(2.0, 4.0, 0.3);
        let mut buffers = [ode.state.as_ptr(), ode.next.as_ptr()];
        buffers.sort_unstable();

        for _ in 0..64 {
            ode.step(0.01).expect("a step runs");
        }

        let mut after = [ode.state.as_ptr(), ode.next.as_ptr()];
        after.sort_unstable();
        assert_eq!(buffers, after, "the two state buffers must be reused");
    }

    #[test]
    fn the_default_start_is_an_even_split() {
        let ode = ReplicatorBuilder::new(hawk_dove(2.0, 4.0))
            .build()
            .expect("valid configuration");

        assert_eq!(ode.current_shares(), [0.5, 0.5]);
        assert_eq!(ode.recorded_generations(), 1);
        assert_eq!(ode.generation(), Generation::ZERO);
    }

    #[test]
    fn an_invalid_initial_state_is_rejected_before_a_trajectory_exists() {
        for shares in [vec![0.5, 0.4], vec![-0.1, 1.1], vec![0.2, 0.3, 0.5]] {
            assert!(
                ReplicatorBuilder::new(hawk_dove(2.0, 4.0))
                    .initial_shares(shares.clone())
                    .build()
                    .is_err(),
                "{shares:?} is not a distribution over two strategies"
            );
        }
    }

    #[test]
    fn each_step_appends_exactly_one_row_and_the_last_row_is_the_state() {
        let mut ode = build(2.0, 4.0, 0.2);

        for expected in 1..=10 {
            ode.step(0.05).expect("a step runs");

            assert_eq!(ode.generation().get(), expected);
            assert_eq!(ode.recorded_generations(), expected as usize + 1);
            assert_eq!(ode.shares_at(ode.generation()), Some(ode.current_shares()));
        }
        assert_eq!(ode.shares_at(Generation::new(11)), None);
        assert_eq!(
            ode.shares_at(Generation::ZERO),
            Some([0.2, 0.8].as_slice()),
            "the initial condition stays where it was recorded"
        );
    }

    #[test]
    fn many_small_steps_agree_with_fewer_large_ones() {
        // Same trajectory, sampled at different resolutions: they must reach
        // the same place, or the curve the UI draws would depend on how often
        // it happened to sample it.
        let mut coarse = build(2.0, 4.0, 0.2);
        let mut fine = build(2.0, 4.0, 0.2);

        coarse.run(100, 0.05).expect("the trajectory stays bounded");
        fine.run(500, 0.01).expect("the trajectory stays bounded");

        let gap = (coarse.current_shares()[0] - fine.current_shares()[0]).abs();
        assert!(gap < 1e-6, "the two resolutions disagree by {gap}");
    }

    #[test]
    fn a_three_strategy_game_runs_through_the_same_integrator() {
        // Issue 4a puts Rock-Paper-Scissors through this, so nothing above may
        // assume two strategies. The interior fixed point of RPS is the even
        // split, and the equation has to leave it exactly there.
        let rps = Game::try_from(vec![
            vec![0.0, -1.0, 1.0],
            vec![1.0, 0.0, -1.0],
            vec![-1.0, 1.0, 0.0],
        ])
        .expect("valid game");
        let mut ode = ReplicatorBuilder::new(rps).build().expect("even split");

        ode.run(1_000, 0.01)
            .expect("the trajectory stays on the simplex");

        for (strategy, &share) in ode.current_shares().iter().enumerate() {
            assert!(
                (share - 1.0 / 3.0).abs() < 1e-12,
                "strategy {strategy} drifted to {share}"
            );
        }
    }
}
