//! Domain error types for the simulation core.
//!
//! Errors are typed enums built with `thiserror`; the wasm boundary in
//! `lib.rs` is the only place that converts them into `JsError`.
//!
//! Every variant carries the offending value as a field rather than a
//! pre-formatted string, so a caller can branch on the numbers (and the wasm
//! boundary can build its own message) instead of parsing Display output.
//!
//! Bounds such as "at least two strategies" travel in a `minimum` field rather
//! than being hardcoded in the message. The constant lives with the code that
//! enforces it, and the message cannot drift out of sync with it.

use thiserror::Error;

/// A payoff matrix that cannot describe a game.
///
/// Returned by the validating constructors in `game`, which check the matrix
/// once at the boundary so the simulation loops can index it unchecked.
#[derive(Debug, Clone, Error)]
pub enum GameError {
    /// A row has a different length than the matrix has rows: a payoff matrix
    /// must be N x N, since every strategy needs a payoff against every other.
    #[error(
        "payoff matrix is not square: row {row} has {found} entries, expected {expected}"
    )]
    NotSquare {
        /// Index of the first row whose length disagrees with the matrix.
        row: usize,
        /// Number of entries that row actually holds.
        found: usize,
        /// Number of entries it needs, i.e. the number of rows.
        expected: usize,
    },

    /// Fewer strategies than a game can be played with.
    #[error("payoff matrix needs at least {minimum} strategies, got {found}")]
    TooFewStrategies {
        /// Number of strategies the matrix describes.
        found: usize,
        /// Smallest number of strategies that makes a game.
        minimum: usize,
    },

    /// More strategies than a `StrategyId` can name, which would leave rows
    /// of the matrix unreachable rather than merely unused.
    #[error("payoff matrix has {found} strategies, at most {maximum} can be addressed")]
    TooManyStrategies {
        /// Number of strategies the matrix describes.
        found: usize,
        /// Largest number of strategies a `StrategyId` can index.
        maximum: usize,
    },

    /// An entry is NaN or infinite. Such an entry poisons every fitness
    /// average downstream, so it is rejected when the matrix is built.
    #[error("payoff matrix entry at row {row}, column {col} is not finite: {value}")]
    NonFiniteEntry {
        /// Row of the offending entry (the focal player's strategy).
        row: usize,
        /// Column of the offending entry (the opponent's strategy).
        col: usize,
        /// The value found there.
        value: f64,
    },
}

/// A simulation that cannot be built or run as configured.
///
/// A simulation owns a game, so an invalid matrix surfaces here too, wrapped
/// rather than flattened: `SimError::Game` keeps the underlying `GameError`
/// available both as a `source` and as a typed value to match on.
#[derive(Debug, Clone, Error)]
pub enum SimError {
    /// Too few agents to run the dynamics. Imitation needs a population an
    /// agent can draw a distinct partner from.
    #[error("population too small: {found} agents, need at least {minimum}")]
    PopulationTooSmall {
        /// Population size requested.
        found: usize,
        /// Smallest population the dynamics are defined for.
        minimum: usize,
    },

    /// The initial shares do not describe the game's strategies one for one.
    #[error("got {found} initial shares for a game with {expected} strategies")]
    ShareCountMismatch {
        /// Number of shares supplied.
        found: usize,
        /// Number of strategies the game has.
        expected: usize,
    },

    /// A share is negative or not finite, so it is not a proportion.
    #[error("initial share for strategy {strategy} is not a proportion: {value}")]
    InvalidShare {
        /// Index of the offending share, which is also its strategy.
        strategy: usize,
        /// The value found there.
        value: f64,
    },

    /// The shares do not add up to a whole population.
    #[error("initial shares sum to {sum}, expected 1 to within {tolerance}")]
    SharesNotNormalised {
        /// What the supplied shares actually add up to.
        sum: f64,
        /// Slack allowed for floating point rounding.
        tolerance: f64,
    },

    /// Selection strength is negative or not finite. Zero is allowed: it is
    /// neutral drift, the baseline selection is measured against.
    #[error("selection strength must be finite and non-negative, got {value}")]
    InvalidSelectionStrength {
        /// The value supplied.
        value: f64,
    },

    /// A generation in which no agent plays anyone. Every score would be
    /// zero, so nothing would distinguish the strategies.
    #[error("each agent must play at least one match per generation")]
    NoMatches,

    /// An agent plays a strategy the game does not define.
    ///
    /// Unreachable through the builder, which sizes the population from the
    /// game's own strategy count. It is reported rather than ignored because
    /// quietly scoring such an agent zero would bias a run invisibly.
    #[error("agent plays strategy {strategy}, but the game has {strategy_count}")]
    UnknownStrategy {
        /// The strategy the agent carries.
        strategy: usize,
        /// How many strategies the game defines.
        strategy_count: usize,
    },

    /// The game the simulation would be played on is invalid.
    #[error("invalid game: {0}")]
    Game(#[from] GameError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn not_square_names_the_offending_row_and_its_length() {
        let msg = GameError::NotSquare {
            row: 1,
            found: 3,
            expected: 2,
        }
        .to_string();
        assert!(msg.contains("row 1"), "{msg}");
        assert!(msg.contains('3'), "{msg}");
        assert!(msg.contains('2'), "{msg}");
    }

    #[test]
    fn too_few_strategies_names_the_count_and_the_minimum() {
        let msg = GameError::TooFewStrategies {
            found: 1,
            minimum: 2,
        }
        .to_string();
        assert!(msg.contains('1'), "{msg}");
        assert!(msg.contains('2'), "{msg}");
    }

    #[test]
    fn non_finite_entry_names_its_position_and_value() {
        let msg = GameError::NonFiniteEntry {
            row: 0,
            col: 1,
            value: f64::NAN,
        }
        .to_string();
        assert!(msg.contains("NaN"), "{msg}");

        let msg = GameError::NonFiniteEntry {
            row: 2,
            col: 3,
            value: f64::NEG_INFINITY,
        }
        .to_string();
        assert!(msg.contains("inf"), "{msg}");
        assert!(msg.contains('2') && msg.contains('3'), "{msg}");
    }

    #[test]
    fn population_too_small_names_the_size_and_the_minimum() {
        let msg = SimError::PopulationTooSmall {
            found: 1,
            minimum: 2,
        }
        .to_string();
        assert!(msg.contains('1'), "{msg}");
        assert!(msg.contains('2'), "{msg}");
    }

    #[test]
    fn sim_error_wraps_a_game_error_without_hiding_it() {
        let cause = GameError::TooFewStrategies {
            found: 0,
            minimum: 2,
        };
        let expected = cause.to_string();

        let err = SimError::from(cause);

        // The wrapper repeats the cause in Display and also exposes it as a
        // `source`, so callers can inspect the typed error instead of parsing text.
        assert!(err.to_string().contains(&expected), "{err}");
        let source = err
            .source()
            .expect("wrapped game error must expose a source");
        assert_eq!(source.to_string(), expected);
    }

    #[test]
    fn question_mark_converts_a_game_error_into_a_sim_error() {
        fn fallible() -> Result<(), SimError> {
            Err(GameError::NotSquare {
                row: 0,
                found: 1,
                expected: 2,
            })?
        }

        assert!(matches!(
            fallible(),
            Err(SimError::Game(GameError::NotSquare { .. }))
        ));
    }
}
