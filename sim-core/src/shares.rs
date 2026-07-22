//! A population's composition, as a distribution over a game's strategies.
//!
//! Every simulation in this crate starts from one of these and reports its
//! history as a sequence of them, so the rules for what counts as a valid
//! composition live here once rather than in each simulation's builder.

use crate::error::SimError;

/// The fraction of a population playing each strategy, indexed by strategy.
///
/// # Invariants
///
/// A `Shares` that exists is a distribution over the strategies of some game:
/// one entry per strategy, every entry a finite non-negative proportion, and
/// the entries summing to 1 to within [`Shares::SUM_TOLERANCE`]. A strategy
/// may be absent, which is what an invasion experiment starts from.
///
/// Validation therefore happens once, where a caller's numbers become a
/// `Shares`, and the simulations that consume one do not re-check it.
#[derive(Debug, Clone, PartialEq)]
pub struct Shares(Vec<f64>);

impl Shares {
    /// Slack allowed when checking that the shares sum to 1.
    ///
    /// Shares are written by hand or moved by a slider, and whether they sum
    /// to exactly 1 in binary floating point depends on the order they happen
    /// to be added in - `0.7 + 0.2 + 0.1` does not, while `0.1 + 0.2 + 0.7`
    /// does. An exact comparison would reject honest input for reasons the
    /// caller cannot see. This is loose enough for accumulated rounding over
    /// a few hundred strategies and far tighter than any real mistake.
    pub const SUM_TOLERANCE: f64 = 1e-9;

    /// Splits the population evenly across `strategy_count` strategies.
    ///
    /// This is what a simulation starts from when the caller expresses no
    /// preference, so it is deliberately the least informative distribution
    /// rather than a favoured strategy.
    pub fn uniform(strategy_count: usize) -> Self {
        Self(vec![1.0 / strategy_count as f64; strategy_count])
    }

    /// Validates `values` as the composition of a population playing a game
    /// with `strategy_count` strategies.
    ///
    /// # Errors
    ///
    /// Returns [`SimError::ShareCountMismatch`] if the values do not describe
    /// the game's strategies one for one, [`SimError::InvalidShare`] if one of
    /// them is negative or not finite, and
    /// [`SimError::SharesNotNormalised`] if they do not add up to a whole
    /// population.
    pub fn checked(values: Vec<f64>, strategy_count: usize) -> Result<Self, SimError> {
        if values.len() != strategy_count {
            return Err(SimError::ShareCountMismatch {
                found: values.len(),
                expected: strategy_count,
            });
        }

        let mut sum = 0.0;
        for (strategy, &value) in values.iter().enumerate() {
            if !value.is_finite() || value < 0.0 {
                return Err(SimError::InvalidShare { strategy, value });
            }
            sum += value;
        }
        if (sum - 1.0).abs() > Self::SUM_TOLERANCE {
            return Err(SimError::SharesNotNormalised {
                sum,
                tolerance: Self::SUM_TOLERANCE,
            });
        }

        Ok(Self(values))
    }

    /// Returns the shares, indexed by strategy.
    pub fn as_slice(&self) -> &[f64] {
        &self.0
    }
}

/// Unwraps the shares into the buffer a simulation then evolves in place.
///
/// One-way on purpose: what comes back out of a running simulation is a state
/// that has moved, and it re-enters the type system as a fresh [`Shares`] only
/// by being validated again.
impl From<Shares> for Vec<f64> {
    fn from(shares: Shares) -> Self {
        shares.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_even_split_is_a_distribution() {
        for strategy_count in [2, 3, 7] {
            let shares = Shares::uniform(strategy_count);
            let values = shares.as_slice();

            assert_eq!(values.len(), strategy_count);
            assert!((values.iter().sum::<f64>() - 1.0).abs() < Shares::SUM_TOLERANCE);
            assert!(values.iter().all(|&v| v == values[0]));
        }
    }

    #[test]
    fn valid_shares_are_kept_exactly_as_given() {
        // The realised composition has to be the one that was asked for, not a
        // renormalised approximation of it: the ODE overlay and the run it is
        // compared against must start from the same numbers.
        let values = vec![0.3, 0.7];
        let shares = Shares::checked(values.clone(), 2).expect("a valid distribution");

        assert_eq!(shares.as_slice(), values);
        assert_eq!(Vec::from(shares), values);
    }

    #[test]
    fn a_strategy_may_start_absent() {
        let shares = Shares::checked(vec![0.0, 1.0], 2).expect("invasions start here");
        assert_eq!(shares.as_slice(), [0.0, 1.0]);
    }

    #[test]
    fn rejects_a_count_that_does_not_match_the_game() {
        let err = Shares::checked(vec![0.2, 0.3, 0.5], 2)
            .expect_err("three shares cannot describe a two-strategy game");
        assert!(
            matches!(
                err,
                SimError::ShareCountMismatch {
                    found: 3,
                    expected: 2
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn rejects_a_negative_or_non_finite_share() {
        for bad in [-0.1, f64::NAN, f64::INFINITY] {
            let err = Shares::checked(vec![bad, 1.0 - bad], 2)
                .expect_err("a share must be a real proportion");
            match err {
                SimError::InvalidShare { strategy, value } => {
                    assert_eq!(strategy, 0);
                    assert!(!value.is_finite() || value < 0.0, "{value}");
                }
                other => panic!("expected an invalid share error, got {other}"),
            }
        }
    }

    #[test]
    fn rejects_shares_that_do_not_sum_to_one() {
        for values in [vec![0.5, 0.4], vec![0.6, 0.6]] {
            let err = Shares::checked(values, 2)
                .expect_err("shares must partition the population");
            assert!(matches!(err, SimError::SharesNotNormalised { .. }), "{err}");
        }
    }

    #[test]
    fn accepts_shares_that_sum_to_one_only_to_within_rounding() {
        // 0.7 + 0.2 + 0.1 lands just under 1.0 in binary floating point, while
        // the same three values added in the other order land exactly on it. A
        // caller cannot be expected to know or control that, so an exact
        // comparison would reject a perfectly ordinary split.
        let values = vec![0.7, 0.2, 0.1];
        assert_ne!(values.iter().sum::<f64>(), 1.0);

        Shares::checked(values, 3)
            .expect("floating point rounding is not a configuration error");
    }
}
