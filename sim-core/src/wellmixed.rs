//! Agent-based well-mixed population: random pairwise matches followed by a
//! stochastic imitation update.
//!
//! Populated by Tasks 1.8 to 1.10 (matching pass, Fermi update, share
//! history).

use crate::game::Game;
use crate::prelude::*;
use crate::rng::Rng;

/// Configuration for a [`WellMixed`] population, validated in [`build`].
///
/// [`build`]: WellMixedBuilder::build
///
/// The game and the population size have no sensible defaults and are given
/// up front; the rest carry documented ones, so the shortest configuration
/// that means anything is also the shortest to write:
///
/// ```
/// use sim_core::game::{Game, HawkDove};
/// use sim_core::wellmixed::WellMixedBuilder;
///
/// let game = Game::try_from(HawkDove { v: 2.0, c: 4.0 })?;
/// let sim = WellMixedBuilder::new(game, 1_000).build()?;
///
/// assert_eq!(sim.population(), 1_000);
/// # Ok::<(), sim_core::error::SimError>(())
/// ```
///
/// Nothing is checked until `build`, so an invalid configuration is one
/// error at one place rather than a different error after each setter.
#[derive(Debug, Clone)]
pub struct WellMixedBuilder {
    game: Game,
    population: usize,
    initial_shares: Option<Vec<f64>>,
    seed: Seed,
    selection_strength: f64,
}

impl WellMixedBuilder {
    /// Slack allowed when checking that the initial shares sum to 1.
    ///
    /// Shares are written by hand or moved by a slider, and whether they sum
    /// to exactly 1 in binary floating point depends on the order they happen
    /// to be added in - `0.7 + 0.2 + 0.1` does not, while `0.1 + 0.2 + 0.7`
    /// does. An exact comparison would reject honest input for reasons the
    /// caller cannot see. This is loose enough for accumulated rounding over
    /// a few hundred strategies and far tighter than any real mistake.
    pub const SHARE_SUM_TOLERANCE: f64 = 1e-9;

    /// Default strength of selection: strong enough that the better strategy
    /// usually wins a comparison, weak enough to leave visible noise.
    pub const DEFAULT_SELECTION_STRENGTH: f64 = 1.0;

    /// Starts a configuration for `population` agents playing `game`.
    ///
    /// Neither argument is validated here. `build` is the single place that
    /// can fail, so a half-built configuration never has to be considered.
    pub fn new(game: Game, population: usize) -> Self {
        Self {
            game,
            population,
            initial_shares: None,
            seed: Seed::new(0),
            selection_strength: Self::DEFAULT_SELECTION_STRENGTH,
        }
    }

    /// Sets the fraction of the population starting on each strategy, indexed
    /// by strategy. Defaults to an even split.
    ///
    /// The shares must be proportions summing to 1; a strategy may start at
    /// zero, which is what an invasion experiment needs.
    pub fn initial_shares(mut self, shares: Vec<f64>) -> Self {
        self.initial_shares = Some(shares);
        self
    }

    /// Sets the seed driving every stochastic decision in the run. Defaults
    /// to seed 0, which is as reproducible as any other.
    pub fn seed(mut self, seed: Seed) -> Self {
        self.seed = seed;
        self
    }

    /// Sets the selection strength beta used by the imitation update.
    ///
    /// Higher values make an agent likelier to copy a better-scoring
    /// neighbour; zero makes every comparison a coin flip, which is neutral
    /// drift. Defaults to [`DEFAULT_SELECTION_STRENGTH`].
    ///
    /// [`DEFAULT_SELECTION_STRENGTH`]: WellMixedBuilder::DEFAULT_SELECTION_STRENGTH
    pub fn selection_strength(mut self, beta: f64) -> Self {
        self.selection_strength = beta;
        self
    }

    /// Validates the configuration and allocates the population.
    ///
    /// Every buffer a generation writes into is sized here, once, so the
    /// per-generation loops can be allocation-free.
    ///
    /// # Errors
    ///
    /// Returns [`SimError`] if the population is too small to imitate anyone,
    /// if the shares do not describe the game's strategies as proportions
    /// summing to 1, or if the selection strength is negative or not finite.
    pub fn build(self) -> Result<WellMixed, SimError> {
        let strategy_count = self.game.strategy_count();

        if self.population < WellMixed::MIN_POPULATION {
            return Err(SimError::PopulationTooSmall {
                found: self.population,
                minimum: WellMixed::MIN_POPULATION,
            });
        }

        if !self.selection_strength.is_finite() || self.selection_strength < 0.0 {
            return Err(SimError::InvalidSelectionStrength {
                value: self.selection_strength,
            });
        }

        let shares = match self.initial_shares {
            Some(shares) => shares,
            None => vec![1.0 / strategy_count as f64; strategy_count],
        };
        if shares.len() != strategy_count {
            return Err(SimError::ShareCountMismatch {
                found: shares.len(),
                expected: strategy_count,
            });
        }

        let mut sum = 0.0;
        for (strategy, &value) in shares.iter().enumerate() {
            if !value.is_finite() || value < 0.0 {
                return Err(SimError::InvalidShare { strategy, value });
            }
            sum += value;
        }
        if (sum - 1.0).abs() > Self::SHARE_SUM_TOLERANCE {
            return Err(SimError::SharesNotNormalised {
                sum,
                tolerance: Self::SHARE_SUM_TOLERANCE,
            });
        }

        let strategies = allocate_population(self.population, &shares);

        Ok(WellMixed {
            game: self.game,
            selection_strength: self.selection_strength,
            rng: Rng::from(self.seed),
            next_strategies: strategies.clone(),
            strategies,
            scores: vec![0.0; self.population],
            generation: Generation::ZERO,
        })
    }
}

/// Hands out whole agents to strategies in the requested proportions.
///
/// Shares are real numbers and agents are not, so some rounding is forced.
/// Each strategy takes its floor, then the leftover agents go to the largest
/// fractional parts (the largest-remainder method), ties broken by the lower
/// strategy id. That fills the population exactly, keeps every realised share
/// within one agent of the request, and is deterministic - drawing the
/// leftovers at random would spend entropy on setup and make two runs with
/// different starting shares incomparable.
///
/// The layout is blocked, all of strategy 0 then all of strategy 1 and so on.
/// Nothing downstream reads it as structure: matches are drawn uniformly at
/// random, so position in the vector carries no meaning.
fn allocate_population(population: usize, shares: &[f64]) -> Vec<StrategyId> {
    let mut counts: Vec<usize> = Vec::with_capacity(shares.len());
    let mut remainders: Vec<(f64, usize)> = Vec::with_capacity(shares.len());

    for (strategy, &share) in shares.iter().enumerate() {
        let exact = share * population as f64;
        let whole = exact.floor();
        // `share <= 1` and `whole <= exact`, so this cannot exceed the
        // population and cannot be negative.
        counts.push(whole as usize);
        remainders.push((exact - whole, strategy));
    }

    let assigned: usize = counts.iter().sum();
    // Each strategy lost less than one agent to flooring, so the shortfall is
    // smaller than the number of strategies and every leftover finds a taker.
    let leftover = population.saturating_sub(assigned);
    remainders.sort_by(|(a_fraction, a_strategy), (b_fraction, b_strategy)| {
        b_fraction
            .total_cmp(a_fraction)
            .then(a_strategy.cmp(b_strategy))
    });
    for &(_, strategy) in remainders.iter().take(leftover) {
        counts[strategy] += 1;
    }

    let mut strategies = Vec::with_capacity(population);
    for (strategy, &count) in counts.iter().enumerate() {
        // `Game` caps a matrix at `Game::MAX_STRATEGIES` = 256 rows, so a
        // strategy index always fits in the byte a `StrategyId` holds.
        let id = StrategyId::new(strategy as u8);
        strategies.extend(std::iter::repeat_n(id, count));
    }
    strategies
}

/// A well-mixed population: every agent is equally likely to meet every
/// other, with no space or network between them.
///
/// This is the finite-population counterpart of the replicator equation, and
/// the contrast between the two is the point of Issue 2a: the same game, the
/// same equilibrium, but a real population of a thousand agents wanders
/// around it instead of settling on it.
///
/// Built through [`WellMixedBuilder`], which is the only place a
/// configuration can be rejected.
#[derive(Debug)]
pub struct WellMixed {
    game: Game,
    selection_strength: f64,
    /// Drives match-ups and imitation decisions from Task 1.8 onward. Held
    /// rather than passed in so a run owns its position in the stream.
    #[cfg_attr(not(test), expect(dead_code, reason = "read by the 1.8 matching pass"))]
    rng: Rng,
    /// One strategy per agent; the population itself.
    strategies: Vec<StrategyId>,
    /// Target buffer for the synchronous update in 1.9. Sized once, never
    /// grown: an update reads `strategies` and writes here, then the two are
    /// swapped, so no agent sees a half-updated population.
    #[cfg_attr(not(test), expect(dead_code, reason = "written by the 1.9 update"))]
    next_strategies: Vec<StrategyId>,
    /// Payoff accumulated by each agent during the current generation.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "written by the 1.8 matching pass")
    )]
    scores: Vec<f64>,
    generation: Generation,
}

impl WellMixed {
    /// Fewest agents the dynamics are defined for: imitation needs someone
    /// other than yourself to copy.
    pub const MIN_POPULATION: usize = 2;

    /// Returns the number of agents.
    pub fn population(&self) -> usize {
        self.strategies.len()
    }

    /// Returns the game being played.
    pub fn game(&self) -> &Game {
        &self.game
    }

    /// Returns the selection strength beta the update uses.
    pub fn selection_strength(&self) -> f64 {
        self.selection_strength
    }

    /// Returns how many generations have been run.
    pub fn generation(&self) -> Generation {
        self.generation
    }

    /// Returns the strategy each agent currently plays.
    ///
    /// Order is an implementation detail: agents have no identity beyond
    /// their strategy and matches are drawn uniformly, so only the
    /// composition of this slice is meaningful.
    pub fn strategies(&self) -> &[StrategyId] {
        &self.strategies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::HawkDove;

    fn hawk_dove() -> Game {
        Game::try_from(HawkDove { v: 2.0, c: 4.0 }).expect("valid game")
    }

    fn three_strategy_game() -> Game {
        Game::try_from(vec![vec![0.0; 3]; 3]).expect("valid game")
    }

    fn count_of(sim: &WellMixed, strategy: StrategyId) -> usize {
        sim.strategies().iter().filter(|&&s| s == strategy).count()
    }

    #[test]
    fn builds_a_population_of_the_requested_size() {
        let sim = WellMixedBuilder::new(hawk_dove(), 1_000)
            .initial_shares(vec![0.5, 0.5])
            .seed(Seed::new(42))
            .selection_strength(1.0)
            .build()
            .expect("valid configuration");

        assert_eq!(sim.population(), 1_000);
        assert_eq!(sim.strategies().len(), 1_000);
        assert_eq!(sim.generation(), Generation::ZERO);
        assert_eq!(sim.selection_strength(), 1.0);
    }

    #[test]
    fn every_scratch_buffer_is_sized_once_at_build_time() {
        // The per-generation loops must never allocate, which they can only
        // guarantee if the buffers they write into already have their final
        // length here.
        let sim = WellMixedBuilder::new(hawk_dove(), 512)
            .build()
            .expect("valid configuration");

        assert_eq!(sim.scores.len(), 512);
        assert_eq!(sim.next_strategies.len(), 512);
        assert_eq!(sim.strategies.len(), 512);
    }

    #[test]
    fn initial_shares_are_realised_exactly_when_they_divide_the_population() {
        let sim = WellMixedBuilder::new(hawk_dove(), 1_000)
            .initial_shares(vec![0.3, 0.7])
            .build()
            .expect("valid configuration");

        assert_eq!(count_of(&sim, HawkDove::HAWK), 300);
        assert_eq!(count_of(&sim, HawkDove::DOVE), 700);
    }

    #[test]
    fn shares_that_do_not_divide_the_population_still_fill_it_exactly() {
        // Thirds of ten agents cannot each be whole. The largest remainders
        // take the leftover agents, so the population is neither short nor
        // over-filled and the realised shares are as close as ten agents
        // allow.
        let sim = WellMixedBuilder::new(three_strategy_game(), 10)
            .initial_shares(vec![1.0 / 3.0, 1.0 / 3.0, 1.0 / 3.0])
            .build()
            .expect("valid configuration");

        let counts: Vec<_> = (0..3).map(|s| count_of(&sim, StrategyId::new(s))).collect();

        assert_eq!(counts.iter().sum::<usize>(), 10);
        assert!(
            counts.iter().all(|&c| c == 3 || c == 4),
            "each third should get 3 or 4 of 10 agents, got {counts:?}"
        );
    }

    #[test]
    fn a_share_of_zero_leaves_a_strategy_absent() {
        let sim = WellMixedBuilder::new(hawk_dove(), 100)
            .initial_shares(vec![0.0, 1.0])
            .build()
            .expect("a strategy may start absent");

        assert_eq!(count_of(&sim, HawkDove::HAWK), 0);
        assert_eq!(count_of(&sim, HawkDove::DOVE), 100);
    }

    #[test]
    fn the_default_split_is_even_across_strategies() {
        let sim = WellMixedBuilder::new(hawk_dove(), 100)
            .build()
            .expect("valid configuration");

        assert_eq!(count_of(&sim, HawkDove::HAWK), 50);
        assert_eq!(count_of(&sim, HawkDove::DOVE), 50);
    }

    #[test]
    fn the_initial_population_is_built_without_touching_the_rng() {
        // Setup must not consume entropy, or changing the initial shares
        // would shift every later draw and make two runs incomparable.
        let mut with_setup = WellMixedBuilder::new(hawk_dove(), 100)
            .initial_shares(vec![0.25, 0.75])
            .seed(Seed::new(7))
            .build()
            .expect("valid configuration");
        let mut fresh = Rng::from(Seed::new(7));

        let from_sim: Vec<_> = (0..16).map(|_| with_setup.rng.next_unit()).collect();
        let from_fresh: Vec<_> = (0..16).map(|_| fresh.next_unit()).collect();

        assert_eq!(from_sim, from_fresh);
    }

    #[test]
    fn rejects_a_population_too_small_to_imitate_anyone() {
        for population in [0, 1] {
            let err = WellMixedBuilder::new(hawk_dove(), population)
                .build()
                .expect_err("imitation needs someone else to copy");
            assert!(
                matches!(
                    err,
                    SimError::PopulationTooSmall { found, minimum }
                        if found == population && minimum == WellMixed::MIN_POPULATION
                ),
                "{err}"
            );
        }
    }

    #[test]
    fn rejects_a_share_per_strategy_count_mismatch() {
        let err = WellMixedBuilder::new(hawk_dove(), 100)
            .initial_shares(vec![0.2, 0.3, 0.5])
            .build()
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
            let err = WellMixedBuilder::new(hawk_dove(), 100)
                .initial_shares(vec![bad, 1.0 - bad])
                .build()
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
        for shares in [vec![0.5, 0.4], vec![0.6, 0.6]] {
            let err = WellMixedBuilder::new(hawk_dove(), 100)
                .initial_shares(shares)
                .build()
                .expect_err("shares must partition the population");
            assert!(matches!(err, SimError::SharesNotNormalised { .. }), "{err}");
        }
    }

    #[test]
    fn accepts_shares_that_sum_to_one_only_to_within_rounding() {
        // 0.7 + 0.2 + 0.1 lands just under 1.0 in binary floating point,
        // while the same three values added in the other order land exactly
        // on it. A caller cannot be expected to know or control that, so an
        // exact comparison would reject a perfectly ordinary split.
        let shares = vec![0.7, 0.2, 0.1];
        assert_ne!(shares.iter().sum::<f64>(), 1.0);

        WellMixedBuilder::new(three_strategy_game(), 99)
            .initial_shares(shares)
            .build()
            .expect("floating point rounding is not a configuration error");
    }

    #[test]
    fn rejects_a_negative_or_non_finite_selection_strength() {
        for bad in [-1.0, f64::NAN, f64::INFINITY] {
            let err = WellMixedBuilder::new(hawk_dove(), 100)
                .selection_strength(bad)
                .build()
                .expect_err("selection strength must be finite and non-negative");
            assert!(
                matches!(err, SimError::InvalidSelectionStrength { .. }),
                "{err}"
            );
        }
    }

    #[test]
    fn accepts_a_selection_strength_of_zero_as_pure_drift() {
        // beta = 0 makes every imitation a coin flip. That is neutral drift,
        // a genuine baseline to compare selection against, not a broken input.
        WellMixedBuilder::new(hawk_dove(), 100)
            .selection_strength(0.0)
            .build()
            .expect("zero selection strength is a valid experiment");
    }

    #[test]
    fn the_same_configuration_always_lays_out_the_same_population() {
        let build = || {
            WellMixedBuilder::new(hawk_dove(), 257)
                .initial_shares(vec![0.42, 0.58])
                .seed(Seed::new(3))
                .build()
                .expect("valid configuration")
        };

        assert_eq!(build().strategies(), build().strategies());
    }
}
