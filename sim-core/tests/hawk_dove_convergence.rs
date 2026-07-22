//! Does the simulation reproduce what the theory predicts?
//!
//! Hawk-Dove with `V < C` has no stable pure strategy. Hawks do badly among
//! hawks and doves do badly among hawks, and the population settles at a hawk
//! *share* of `V / C`. These tests assert that number, from the public API,
//! at a fixed seed - the claim Issue 1 exists to demonstrate.
//!
//! Kept apart from the unit tests in `wellmixed.rs`: those pin the mechanics
//! of a step, these pin the behaviour of a run.

use sim_core::game::{Game, HawkDove};
use sim_core::prelude::*;
use sim_core::wellmixed::{WellMixed, WellMixedBuilder};

/// Large enough for a share to be a meaningful average, small enough to stay
/// visibly noisy - the same tension the UI's default has to strike.
const POPULATION: usize = 1_000;

/// Long enough to leave any starting point and settle.
const GENERATIONS: usize = 2_000;

/// How many trailing generations the measured share averages over. Averaging
/// is the point: a finite population never sits *on* its equilibrium, it
/// wanders around it, so a single generation proves nothing either way.
const TAIL: usize = 200;

/// Slack between the measured share and `V / C`, wide enough to absorb the
/// wandering of a thousand agents and far tighter than the gap between the
/// equilibria being told apart (0.5, 0.75 and 0.333).
const TOLERANCE: f64 = 0.05;

fn run(v: f64, c: f64, initial_hawk_share: f64, seed: u64) -> WellMixed {
    let game = Game::try_from(HawkDove { v, c }).expect("V and C are finite");
    let mut sim = WellMixedBuilder::new(game, POPULATION)
        .initial_shares(vec![initial_hawk_share, 1.0 - initial_hawk_share])
        .seed(Seed::new(seed))
        .build()
        .expect("valid configuration");

    for _ in 0..GENERATIONS {
        sim.step().expect("a generation runs");
    }
    sim
}

/// Mean hawk share over the last [`TAIL`] generations of the history.
fn settled_hawk_share(sim: &WellMixed) -> f64 {
    let history = sim.share_history();
    let hawk_shares: Vec<f64> = history
        .chunks_exact(2)
        .map(|generation| generation[0])
        .collect();
    let tail = &hawk_shares[hawk_shares.len() - TAIL..];
    tail.iter().sum::<f64>() / TAIL as f64
}

#[test]
fn the_hawk_share_settles_at_v_over_c() {
    // Three pairs rather than one, so the test pins the formula V/C and not a
    // single memorised number: an implementation that always converged to 0.5
    // would pass the first case and fail the others.
    for (v, c) in [(2.0, 4.0), (1.0, 3.0), (3.0, 4.0)] {
        let sim = run(v, c, 0.5, 20_260_722);
        let measured = settled_hawk_share(&sim);
        let predicted = v / c;

        assert!(
            (measured - predicted).abs() < TOLERANCE,
            "V={v}, C={c}: predicted a hawk share of {predicted}, measured {measured}"
        );
    }
}

#[test]
fn the_equilibrium_is_reached_from_either_side() {
    // V/C is an attractor, not just a fixed point: hawks invade a population
    // of doves and collapse in a population of hawks, and both journeys end
    // in the same place. A run that merely stayed where it started would pass
    // the test above and fail this one.
    let (v, c) = (2.0, 4.0);
    let predicted = v / c;

    for initial_hawk_share in [0.05, 0.95] {
        let sim = run(v, c, initial_hawk_share, 4_242);
        let measured = settled_hawk_share(&sim);

        assert!(
            (measured - predicted).abs() < TOLERANCE,
            "starting from {initial_hawk_share}: predicted {predicted}, measured {measured}"
        );
    }
}

#[test]
fn a_dominant_strategy_takes_the_whole_population() {
    // V > C inverts the story: fighting pays, hawk beats dove against every
    // opponent, and there is no interior equilibrium to settle at. V/C would
    // be 3 here, which is not a share at all - the right answer is 1.
    let sim = run(6.0, 2.0, 0.5, 77);
    let measured = settled_hawk_share(&sim);

    assert!(
        measured > 0.99,
        "hawk should sweep the population when V > C, measured {measured}"
    );
}

#[test]
fn every_generation_of_a_long_run_is_a_distribution() {
    // The invariant that has to hold whatever the dynamics do: shares are
    // proportions of one population, so each generation sums to 1.
    let sim = run(2.0, 4.0, 0.3, 909);
    let history = sim.share_history();

    assert_eq!(history.len(), (GENERATIONS + 1) * 2);
    for (generation, shares) in history.chunks_exact(2).enumerate() {
        let sum: f64 = shares.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-12,
            "generation {generation} sums to {sum}, not 1"
        );
        assert!(
            shares.iter().all(|share| (0.0..=1.0).contains(share)),
            "generation {generation} has a share outside [0, 1]: {shares:?}"
        );
    }
}

#[test]
fn the_same_seed_reproduces_the_whole_run() {
    // Every claim above is a claim about one seed, which only means anything
    // if that seed replays exactly.
    let history = |seed| run(2.0, 4.0, 0.5, seed).share_history().to_vec();

    assert_eq!(history(1_234), history(1_234));
    assert_ne!(history(1_234), history(1_235));
}
