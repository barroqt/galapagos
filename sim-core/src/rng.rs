//! Seedable randomness. The only source of entropy in the crate.
//!
//! Every stochastic API takes an explicit seed, so the same seed and the same
//! parameters reproduce a run exactly. Nothing in this crate may call
//! `thread_rng`.
//!
//! That last rule is enforced by the build rather than by discipline: `rand`
//! is a dependency with `default-features = false`, so `thread_rng` is not
//! compiled in and reaching for it is a compile error, not a review comment.

use crate::prelude::*;
use rand::rngs::SmallRng;
use rand::{Rng as _, SeedableRng};

/// The crate's only random number generator, seeded explicitly.
///
/// # Why the surface is this small
///
/// Two methods cover every stochastic decision the simulations make: pick a
/// random agent or cell ([`Rng::next_index`]), and accept something with a
/// given probability ([`Rng::next_unit`]). Keeping it to those two means the
/// number of draws a step consumes is easy to reason about, which is what
/// makes a run replayable - a helper that sometimes drew twice would desync a
/// replay in a way that is very hard to see.
///
/// Deliberately not `Clone`: two generators sharing a position would produce
/// the same "random" stream twice, and correlated draws in a simulation are
/// far harder to spot than an outright failure.
///
/// # Reproducibility
///
/// The same [`Seed`] and the same sequence of calls always produce the same
/// values *within a build*. The underlying `SmallRng` algorithm is allowed to
/// change between `rand` releases, so seeds are reproducible across machines
/// running the same binary, not across upgrades of the dependency. It is
/// chosen anyway: it is small and fast in WASM, and nothing here is
/// cryptographic.
#[derive(Debug)]
pub struct Rng {
    inner: SmallRng,
}

/// Starts a generator at the position a seed names.
impl From<Seed> for Rng {
    fn from(seed: Seed) -> Self {
        Self {
            inner: SmallRng::seed_from_u64(seed.get()),
        }
    }
}

impl Rng {
    /// Draws a uniform index in `0..len`, or `None` when there is nothing to
    /// index.
    ///
    /// The empty case consumes no entropy, so a caller that asks about an
    /// empty population does not shift every later draw and change the rest
    /// of the run.
    ///
    /// The distribution is uniform without modulo bias: `gen_range` rejects
    /// and redraws rather than folding the range, which is why a draw is not
    /// a fixed amount of entropy.
    pub fn next_index(&mut self, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        Some(self.inner.gen_range(0..len))
    }

    /// Draws a uniform `f64` in `[0, 1)`.
    ///
    /// Half-open at the top: comparing `next_unit() < p` accepts with
    /// probability exactly `p`, including `p = 1`.
    pub fn next_unit(&mut self) -> f64 {
        self.inner.gen()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Draws a fixed interleaving of both methods. Interleaved rather than
    /// grouped because a stream is only reproducible if *call order* drives
    /// it, which grouped draws would not catch.
    fn sample(seed: u64) -> Vec<f64> {
        let mut rng = Rng::from(Seed::new(seed));
        let mut out = Vec::new();
        for _ in 0..64 {
            out.push(rng.next_unit());
            let index = rng.next_index(7).expect("7 is not empty");
            out.push(index as f64);
        }
        out
    }

    #[test]
    fn the_same_seed_replays_the_same_stream() {
        // Compared against a second instance rather than against hardcoded
        // values: `SmallRng`'s algorithm is explicitly allowed to change
        // between `rand` releases, so golden numbers would pin the dependency
        // rather than the property we care about.
        assert_eq!(sample(12345), sample(12345));
        assert_eq!(sample(0), sample(0));
    }

    #[test]
    fn different_seeds_diverge() {
        assert_ne!(sample(1), sample(2));
    }

    #[test]
    fn an_empty_range_has_no_index_to_draw() {
        let mut rng = Rng::from(Seed::new(7));
        assert_eq!(rng.next_index(0), None);
    }

    #[test]
    fn a_failed_draw_does_not_disturb_the_stream() {
        // A population that briefly empties must not shift every later draw,
        // or reproducibility would depend on how often that happened.
        let mut with_failures = Rng::from(Seed::new(99));
        let mut without = Rng::from(Seed::new(99));

        let drawn: Vec<_> = (0..8)
            .map(|_| {
                assert_eq!(with_failures.next_index(0), None);
                with_failures.next_unit()
            })
            .collect();
        let expected: Vec<_> = (0..8).map(|_| without.next_unit()).collect();

        assert_eq!(drawn, expected);
    }

    #[test]
    fn indices_stay_inside_the_range_and_reach_both_ends() {
        let mut rng = Rng::from(Seed::new(2024));
        let len = 5;
        let mut seen = [false; 5];

        for _ in 0..1_000 {
            let index = rng.next_index(len).expect("range is not empty");
            assert!(index < len, "{index} is outside 0..{len}");
            seen[index] = true;
        }

        assert!(seen.iter().all(|&hit| hit), "every index must be reachable");
    }

    #[test]
    fn unit_draws_stay_in_the_half_open_unit_interval() {
        // The Fermi update in 1.9 compares a draw against a probability, so a
        // draw of exactly 1.0 would make a probability-1 event fail to fire.
        let mut rng = Rng::from(Seed::new(4321));
        for _ in 0..10_000 {
            let value = rng.next_unit();
            assert!((0.0..1.0).contains(&value), "{value} is outside [0, 1)");
        }
    }

    #[test]
    fn a_single_index_range_needs_no_entropy_but_stays_in_range() {
        let mut rng = Rng::from(Seed::new(5));
        assert_eq!(rng.next_index(1), Some(0));
    }
}
