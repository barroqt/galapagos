//! Payoff matrices and the built-in games defined over them.
//!
//! Populated by Task 1.5 (`HawkDove`). Later issues add Rock-Paper-Scissors
//! and Stag Hunt here.

use crate::prelude::*;

/// A symmetric N-strategy game, held as its payoff matrix.
///
/// # Reading the matrix
///
/// **The row is the focal player and the column is the opponent**, and
/// `payoff(focal, opponent)` is what the focal player collects from that
/// meeting. Every module in this crate reads it that way; a transposed read
/// would silently invert which strategy is doing well, so the convention is
/// pinned by test rather than only by this paragraph.
///
/// # Layout
///
/// Entries live in one flat row-major `Vec<f64>` rather than a `Vec<Vec<f64>>`
/// so a lookup is one bounds-checked index into contiguous memory instead of
/// chasing a pointer per row. The nested form appears only at construction,
/// where it is the natural shape to write a matrix in.
///
/// # Invariants
///
/// A `Game` that exists is playable: it has between [`Game::MIN_STRATEGIES`]
/// and [`Game::MAX_STRATEGIES`] strategies, it is square, and every entry is
/// finite. Construction is the only place that can fail, which is what lets
/// the per-generation loops index it without revalidating, and what makes the
/// derived `PartialEq` meaningful - no reachable entry is NaN.
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    /// `strategy_count * strategy_count` entries, row-major.
    payoffs: Vec<f64>,
    /// Cached side length; `payoffs.len()` is its square.
    strategy_count: usize,
}

impl Game {
    /// Fewest strategies that still make a game: with one strategy there is
    /// nothing to select between and no dynamics to run.
    pub const MIN_STRATEGIES: usize = 2;

    /// Most strategies a [`StrategyId`] can name, since it is one byte wide.
    pub const MAX_STRATEGIES: usize = u8::MAX as usize + 1;

    /// Returns how many strategies the game is played with.
    pub fn strategy_count(&self) -> usize {
        self.strategy_count
    }

    /// Reports whether `strategy` is one of this game's strategies.
    ///
    /// A [`StrategyId`] is not bound to a particular game, so a population
    /// carrying ids from elsewhere is checked here once, at the boundary,
    /// rather than in the loops that then index the matrix.
    pub fn contains(&self, strategy: StrategyId) -> bool {
        strategy.index() < self.strategy_count
    }

    /// Returns everything a player of `focal` can collect, indexed by the
    /// opponent's strategy, or `None` if `focal` is not in this game.
    ///
    /// This is the shape the replicator fitness pass wants: one lookup per
    /// strategy instead of one per pair.
    pub fn row(&self, focal: StrategyId) -> Option<&[f64]> {
        let start = focal.index().checked_mul(self.strategy_count)?;
        self.payoffs
            .get(start..start.checked_add(self.strategy_count)?)
    }

    /// Returns the payoff *to* the focal player when `focal` meets
    /// `opponent`, or `None` if either strategy is not in this game.
    ///
    /// `None` rather than a panic keeps a mismatched id a value the caller
    /// handles rather than a crash inside a simulation step, and rather than
    /// a `Result` because the reason is never in doubt: that id is not in
    /// this game.
    pub fn payoff(&self, focal: StrategyId, opponent: StrategyId) -> Option<f64> {
        self.row(focal)?.get(opponent.index()).copied()
    }
}

/// Builds a game from a matrix written the way it reads on paper, one row per
/// focal strategy, validating it once so nothing downstream has to.
impl TryFrom<Vec<Vec<f64>>> for Game {
    type Error = GameError;

    fn try_from(rows: Vec<Vec<f64>>) -> Result<Self, Self::Error> {
        let strategy_count = rows.len();
        if strategy_count < Self::MIN_STRATEGIES {
            return Err(GameError::TooFewStrategies {
                found: strategy_count,
                minimum: Self::MIN_STRATEGIES,
            });
        }
        if strategy_count > Self::MAX_STRATEGIES {
            return Err(GameError::TooManyStrategies {
                found: strategy_count,
                maximum: Self::MAX_STRATEGIES,
            });
        }

        let mut payoffs = Vec::with_capacity(strategy_count * strategy_count);
        for (row, entries) in rows.into_iter().enumerate() {
            if entries.len() != strategy_count {
                return Err(GameError::NotSquare {
                    row,
                    found: entries.len(),
                    expected: strategy_count,
                });
            }
            for (col, value) in entries.into_iter().enumerate() {
                if !value.is_finite() {
                    return Err(GameError::NonFiniteEntry { row, col, value });
                }
                payoffs.push(value);
            }
        }

        Ok(Self {
            payoffs,
            strategy_count,
        })
    }
}

/// The Hawk-Dove game: two animals contest a resource worth `v`, and a fight
/// between two hawks costs `c`.
///
/// Two hawks fight and split the outcome, `(v - c) / 2`. A hawk facing a dove
/// takes the whole resource, `v`, while the dove retreats with `0`. Two doves
/// share it, `v / 2`.
///
/// # Why the game matters
///
/// When `c > v` neither pure strategy is stable: hawks do badly in a
/// population of hawks, doves do badly against hawks, and the population
/// settles at a hawk *share* of `v / c` rather than on a single strategy.
/// That mixed equilibrium is what the simulations in this crate reproduce.
///
/// `v > c` is an ordinary game too, not a misconfiguration: fighting simply
/// pays, hawk dominates, and the population goes to all-hawk. The conversion
/// deliberately does not reject it.
///
/// # Validation
///
/// The fields are plain parameters with no constraint of their own, so they
/// are checked where they become payoffs: [`Game`] rejects any `v`/`c` pair
/// whose entries are not finite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HawkDove {
    /// Value of the contested resource.
    pub v: f64,
    /// Cost of losing a fight between two hawks.
    pub c: f64,
}

impl HawkDove {
    /// Escalates and fights. Row and column 0 of the payoff matrix.
    pub const HAWK: StrategyId = StrategyId::new(0);

    /// Displays and retreats rather than fighting. Row and column 1.
    pub const DOVE: StrategyId = StrategyId::new(1);
}

/// Builds the 2x2 Hawk-Dove payoff matrix, laid out row = focal player as
/// every [`Game`] is.
impl TryFrom<HawkDove> for Game {
    type Error = GameError;

    fn try_from(HawkDove { v, c }: HawkDove) -> Result<Self, Self::Error> {
        //          vs HAWK      vs DOVE
        // HAWK    (v - c) / 2        v
        // DOVE              0    v / 2
        Game::try_from(vec![vec![(v - c) / 2.0, v], vec![0.0, v / 2.0]])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deliberately asymmetric: every entry is distinct, so a transposed or
    /// column-major read cannot pass by coincidence.
    fn asymmetric_3x3() -> Vec<Vec<f64>> {
        vec![
            vec![0.0, 1.0, 2.0],
            vec![10.0, 11.0, 12.0],
            vec![20.0, 21.0, 22.0],
        ]
    }

    fn game_from(rows: Vec<Vec<f64>>) -> Game {
        Game::try_from(rows).expect("matrix is valid")
    }

    #[test]
    fn row_is_the_focal_player() {
        // Row 0 against column 1 is what a player of strategy 0 collects when
        // it meets strategy 1. Reading the matrix transposed would give 10.0.
        let game = game_from(asymmetric_3x3());
        assert_eq!(
            game.payoff(StrategyId::new(0), StrategyId::new(1)),
            Some(1.0)
        );
        assert_eq!(
            game.payoff(StrategyId::new(1), StrategyId::new(0)),
            Some(10.0)
        );
    }

    #[test]
    fn every_entry_keeps_its_position_through_the_flattening() {
        let rows = asymmetric_3x3();
        let game = game_from(rows.clone());

        for (i, row) in rows.iter().enumerate() {
            for (j, &expected) in row.iter().enumerate() {
                let payoff =
                    game.payoff(StrategyId::new(i as u8), StrategyId::new(j as u8));
                assert_eq!(payoff, Some(expected), "entry [{i}][{j}]");
            }
        }
        assert_eq!(game.strategy_count(), 3);
    }

    #[test]
    fn row_view_matches_entry_lookup() {
        let game = game_from(asymmetric_3x3());
        assert_eq!(
            game.row(StrategyId::new(1)),
            Some([10.0, 11.0, 12.0].as_slice())
        );
    }

    #[test]
    fn ids_outside_the_game_are_absent_rather_than_a_panic() {
        let game = game_from(asymmetric_3x3());
        let inside = StrategyId::new(2);
        let outside = StrategyId::new(3);

        assert!(game.contains(inside));
        assert!(!game.contains(outside));
        assert_eq!(game.row(outside), None);
        assert_eq!(game.payoff(outside, inside), None);
        assert_eq!(game.payoff(inside, outside), None);
    }

    #[test]
    fn rejects_fewer_than_two_strategies() {
        for rows in [vec![], vec![vec![1.0]]] {
            let found = rows.len();
            let err = Game::try_from(rows).expect_err("a game needs two strategies");
            assert!(
                matches!(
                    err,
                    GameError::TooFewStrategies { found: f, minimum }
                        if f == found && minimum == Game::MIN_STRATEGIES
                ),
                "{err}"
            );
        }
    }

    #[test]
    fn rejects_more_strategies_than_a_strategy_id_can_name() {
        let n = Game::MAX_STRATEGIES + 1;
        let err = Game::try_from(vec![vec![0.0; n]; n]).expect_err("too many strategies");
        assert!(
            matches!(
                err,
                GameError::TooManyStrategies { found, maximum }
                    if found == n && maximum == Game::MAX_STRATEGIES
            ),
            "{err}"
        );
    }

    #[test]
    fn rejects_a_short_or_long_row_and_names_the_first_one() {
        // Row 0 is well formed, so the error must name row 1 and not just the
        // first row it happened to visit.
        let short = vec![vec![1.0, 2.0, 3.0], vec![4.0], vec![5.0, 6.0, 7.0]];
        let err = Game::try_from(short).expect_err("rows must be as long as the matrix");
        assert!(
            matches!(
                err,
                GameError::NotSquare {
                    row: 1,
                    found: 1,
                    expected: 3
                }
            ),
            "{err}"
        );

        let long = vec![vec![1.0, 2.0, 9.0], vec![3.0, 4.0]];
        let err = Game::try_from(long).expect_err("rows must be as long as the matrix");
        assert!(
            matches!(
                err,
                GameError::NotSquare {
                    row: 0,
                    found: 3,
                    expected: 2
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn rejects_non_finite_entries() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let rows = vec![vec![1.0, 2.0], vec![3.0, bad]];
            let err = Game::try_from(rows).expect_err("entries must be finite");
            match err {
                GameError::NonFiniteEntry { row, col, value } => {
                    assert_eq!((row, col), (1, 1));
                    assert!(!value.is_finite(), "{value}");
                }
                other => panic!("expected a non-finite entry error, got {other}"),
            }
        }
    }

    #[test]
    fn hawk_dove_writes_all_four_entries_in_the_right_places() {
        // V=2, C=4 gives four distinct payoffs (-1, 2, 0, 1), so no pair of
        // entries can be swapped without the test noticing.
        let game = Game::try_from(HawkDove { v: 2.0, c: 4.0 }).expect("valid game");
        let (hawk, dove) = (HawkDove::HAWK, HawkDove::DOVE);

        assert_eq!(game.strategy_count(), 2);
        assert_eq!(game.payoff(hawk, hawk), Some(-1.0), "H vs H = (V-C)/2");
        assert_eq!(game.payoff(hawk, dove), Some(2.0), "H vs D = V");
        assert_eq!(game.payoff(dove, hawk), Some(0.0), "D vs H = 0");
        assert_eq!(game.payoff(dove, dove), Some(1.0), "D vs D = V/2");
    }

    #[test]
    fn hawk_dove_is_not_transposed() {
        // The asymmetric pair is the whole game: a hawk takes the resource
        // from a dove, never the other way round. A transposed matrix would
        // still be square, still be finite, and quietly invert the dynamics.
        let game = Game::try_from(HawkDove { v: 2.0, c: 4.0 }).expect("valid game");
        let hawk_over_dove = game
            .payoff(HawkDove::HAWK, HawkDove::DOVE)
            .expect("hawk meets dove");
        let dove_under_hawk = game
            .payoff(HawkDove::DOVE, HawkDove::HAWK)
            .expect("dove meets hawk");

        assert!(
            hawk_over_dove > dove_under_hawk,
            "hawk must collect V={hawk_over_dove} against a dove, which collects \
             {dove_under_hawk}"
        );
    }

    #[test]
    fn hawk_is_zero_and_dove_is_one_in_the_game_they_index() {
        let game = Game::try_from(HawkDove { v: 2.0, c: 4.0 }).expect("valid game");
        assert_eq!(HawkDove::HAWK.get(), 0);
        assert_eq!(HawkDove::DOVE.get(), 1);
        assert!(game.contains(HawkDove::HAWK));
        assert!(game.contains(HawkDove::DOVE));
    }

    #[test]
    fn a_cheap_fight_is_still_a_game() {
        // V > C: fighting pays, so hawk strictly dominates and the population
        // goes to all-hawk. That is a real prediction, not a broken input, so
        // the conversion must accept it.
        let game = Game::try_from(HawkDove { v: 6.0, c: 2.0 }).expect("V > C is valid");
        assert_eq!(game.payoff(HawkDove::HAWK, HawkDove::HAWK), Some(2.0));
        assert!(
            game.payoff(HawkDove::HAWK, HawkDove::HAWK)
                > game.payoff(HawkDove::DOVE, HawkDove::HAWK),
            "hawk dominates when V > C"
        );
    }

    #[test]
    fn hawk_dove_rejects_parameters_that_produce_a_non_finite_payoff() {
        for (v, c) in [
            (f64::NAN, 4.0),
            (2.0, f64::NAN),
            (f64::INFINITY, f64::INFINITY),
            (f64::MAX, -f64::MAX),
        ] {
            let err = Game::try_from(HawkDove { v, c })
                .expect_err("non-finite parameters cannot make a game");
            assert!(matches!(err, GameError::NonFiniteEntry { .. }), "{err}");
        }
    }

    #[test]
    fn a_valid_game_never_holds_a_nan_so_equality_is_meaningful() {
        // Rejecting non-finite entries at construction is what lets `Game`
        // derive `PartialEq`: no reachable value compares unequal to itself.
        let game = game_from(asymmetric_3x3());
        assert_eq!(game, game.clone());
    }
}
