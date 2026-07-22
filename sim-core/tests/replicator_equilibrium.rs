//! Does the deterministic limit of the dynamics land where the theory says?
//!
//! The replicator equation is what an infinite, well-mixed population does.
//! For Hawk-Dove with `V < C` it has one interior fixed point, `V / C`, and it
//! is an attractor: every interior starting share ends there. These tests
//! assert that from the public API, which is also the curve the UI overlays on
//! the agent-based run.
//!
//! Kept apart from the unit tests in `replicator.rs`: those pin the mechanics
//! of the integrator, these pin the behaviour of a run.

use sim_core::game::{Game, HawkDove};
use sim_core::replicator::{Replicator, ReplicatorBuilder};

/// Integration step. Small enough that the fourth-order error over a run is
/// far below [`TOLERANCE`], large enough that a run is a few thousand steps
/// rather than a few million.
const DT: f64 = 0.01;

/// Steps per run, so each run covers `STEPS * DT = 200` units of time.
///
/// Approach to the interior equilibrium is exponential: linearising
/// `x' = x(1-x)(C/2)(V/C - x)` at `x* = V/C` gives a rate of `x*(1-x*)C/2`.
/// The slowest case tested below is V=1, C=3, whose rate is 1/3, so 200 time
/// units is some 66 e-foldings and what is left of the initial displacement
/// is `1e-29`. The tolerance below is deliberately not the thing doing the
/// work here; the run is long enough that it does not have to be.
const STEPS: usize = 20_000;

/// Slack between the settled share and `V / C`.
///
/// Tight on purpose. `V / C` is an exact fixed point of the *integrator* as
/// well as of the equation - every RK4 stage is zero there, so the truncation
/// error that scales with `DT` vanishes at the answer, and only rounding is
/// left. A tolerance loose enough to hide a wrong equilibrium would make the
/// test meaningless.
const TOLERANCE: f64 = 1e-9;

fn run(v: f64, c: f64, initial_hawk_share: f64) -> Replicator {
    let game = Game::try_from(HawkDove { v, c }).expect("V and C are finite");
    let mut ode = ReplicatorBuilder::new(game)
        .initial_shares(vec![initial_hawk_share, 1.0 - initial_hawk_share])
        .build()
        .expect("valid configuration");

    ode.run(STEPS, DT)
        .expect("the trajectory stays on the simplex");
    ode
}

fn hawk_share(ode: &Replicator) -> f64 {
    ode.current_shares()[HawkDove::HAWK.index()]
}

#[test]
fn the_trajectory_converges_to_v_over_c_from_anywhere_inside() {
    // Two games rather than one, so the test pins the formula V/C and not a
    // single memorised number, and five starting points on both sides of each
    // equilibrium, so it pins an attractor and not merely a fixed point.
    for (v, c) in [(2.0, 4.0), (1.0, 3.0)] {
        let predicted = v / c;

        for initial_hawk_share in [0.01, 0.3, 0.5, 0.9, 0.99] {
            let measured = hawk_share(&run(v, c, initial_hawk_share));

            assert!(
                (measured - predicted).abs() < TOLERANCE,
                "V={v}, C={c} from {initial_hawk_share}: predicted {predicted}, \
                 measured {measured}"
            );
        }
    }
}

#[test]
fn the_pure_populations_are_fixed_points() {
    // `x' = x(1-x)(...)` is zero at both ends, and the n-strategy form this is
    // written in has to reproduce that exactly rather than nearly: a strategy
    // at share zero has a growth rate of zero times something, and a strategy
    // at share one is the whole population, so its fitness *is* the mean.
    // Anything else would let the integrator resurrect an extinct strategy.
    for initial_hawk_share in [0.0, 1.0] {
        let ode = run(2.0, 4.0, initial_hawk_share);

        assert_eq!(
            ode.current_shares(),
            [initial_hawk_share, 1.0 - initial_hawk_share],
            "a pure population must not move"
        );
    }
}

#[test]
fn a_dominant_strategy_sweeps_the_population() {
    // V > C: fighting pays, hawk beats dove against every opponent, and there
    // is no interior equilibrium. V/C would be 3 here, which is not a share at
    // all - the right answer is 1.
    let measured = hawk_share(&run(6.0, 2.0, 0.5));

    assert!(
        (measured - 1.0).abs() < TOLERANCE,
        "hawk should sweep the population when V > C, measured {measured}"
    );
}

#[test]
fn the_trajectory_has_the_layout_the_agent_based_history_has() {
    // The UI overlays the two buffers directly, so this one has to be flat,
    // generation-major, and one row longer than the number of steps run -
    // exactly what `WellMixed::share_history` produces.
    let ode = run(2.0, 4.0, 0.3);
    let history = ode.share_history();

    assert_eq!(ode.recorded_generations(), STEPS + 1);
    assert_eq!(history.len(), (STEPS + 1) * 2);
    assert_eq!(&history[..2], [0.3, 0.7], "step 0 is the initial condition");

    for (step, shares) in history.chunks_exact(2).enumerate() {
        let sum: f64 = shares.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-12,
            "step {step} sums to {sum}, not 1"
        );
        assert!(
            shares.iter().all(|share| (0.0..=1.0).contains(share)),
            "step {step} has a share outside [0, 1]: {shares:?}"
        );
    }
}

#[test]
fn the_same_configuration_reproduces_the_whole_trajectory() {
    // No seed appears anywhere above because there is nothing stochastic here.
    // That is a claim worth asserting: the ODE is the run the agent-based
    // simulation is compared *against*, so it has to be the same curve every
    // time.
    let trajectory = |initial| run(2.0, 4.0, initial).share_history().to_vec();

    assert_eq!(trajectory(0.42), trajectory(0.42));
    assert_ne!(trajectory(0.42), trajectory(0.43));
}
