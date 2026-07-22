//! Agent-based well-mixed population: random pairwise matches followed by a
//! stochastic imitation update.
//!
//! Populated by Tasks 1.8 to 1.10 (matching pass, Fermi update, share
//! history).

use crate::game::Game;
use crate::prelude::*;
use crate::rng::Rng;
use std::num::NonZeroUsize;

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
    matches_per_agent: usize,
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

    /// Default number of opponents each agent meets per generation.
    ///
    /// A score is a sample of what a strategy earns against the current
    /// population, and this is how many draws that sample averages over. Too
    /// few and selection acts mostly on luck; too many and a finite
    /// population stops differing from the replicator equation, which is the
    /// contrast Issue 2a exists to show.
    pub const DEFAULT_MATCHES_PER_AGENT: usize = 10;

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
            matches_per_agent: Self::DEFAULT_MATCHES_PER_AGENT,
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

    /// Sets how many opponents each agent meets per generation. Defaults to
    /// [`DEFAULT_MATCHES_PER_AGENT`].
    ///
    /// Must be at least one: with no matches every score is zero, so nothing
    /// distinguishes the strategies and the run carries no information.
    ///
    /// [`DEFAULT_MATCHES_PER_AGENT`]: WellMixedBuilder::DEFAULT_MATCHES_PER_AGENT
    pub fn matches_per_agent(mut self, matches: usize) -> Self {
        self.matches_per_agent = matches;
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

        let matches_per_agent =
            NonZeroUsize::new(self.matches_per_agent).ok_or(SimError::NoMatches)?;

        let strategies = allocate_population(self.population, &shares);

        let mut sim = WellMixed {
            game: self.game,
            selection_strength: self.selection_strength,
            matches_per_agent,
            rng: Rng::from(self.seed),
            next_strategies: strategies.clone(),
            strategies,
            scores: vec![0.0; self.population],
            history: Vec::new(),
            generation: Generation::ZERO,
        };
        // Generation 0 is the population as configured, recorded before
        // anything runs so a run and its analytic overlay start from the same
        // point rather than one generation apart.
        sim.record_shares()?;
        Ok(sim)
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
    /// How many opponents each agent meets per generation. `NonZeroUsize`
    /// because a generation with no matches scores everyone zero, so the
    /// builder rejects it and nothing below has to consider the case.
    matches_per_agent: NonZeroUsize,
    /// Drives match-ups and imitation decisions. Held rather than passed in
    /// so a run owns its position in the stream.
    rng: Rng,
    /// One strategy per agent; the population itself.
    strategies: Vec<StrategyId>,
    /// Target buffer for the synchronous update in 1.9. Sized once, never
    /// grown: an update reads `strategies` and writes here, then the two are
    /// swapped, so no agent sees a half-updated population.
    next_strategies: Vec<StrategyId>,
    /// Payoff each agent collected during the current generation. Overwritten
    /// in full by every matching pass, never accumulated across generations.
    scores: Vec<f64>,
    /// Strategy shares per generation, generation-major and flat. See
    /// [`WellMixed::share_history`] for the layout and why it is one buffer.
    history: Vec<f64>,
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

    /// Returns how many opponents each agent meets per generation.
    pub fn matches_per_agent(&self) -> NonZeroUsize {
        self.matches_per_agent
    }

    /// Returns the strategy each agent currently plays.
    ///
    /// Order is an implementation detail: agents have no identity beyond
    /// their strategy and matches are drawn uniformly, so only the
    /// composition of this slice is meaningful.
    pub fn strategies(&self) -> &[StrategyId] {
        &self.strategies
    }

    /// Plays one generation of matches, writing each agent's total payoff
    /// into the score buffer.
    ///
    /// Every agent initiates exactly `matches_per_agent` matches against
    /// uniformly drawn opponents, and only the initiator banks the payoff.
    /// Crediting both sides would give an agent that happened to be drawn
    /// often a larger total for no reason of its own; with a fixed number of
    /// matches each, totals are directly comparable, which is exactly what
    /// the imitation update needs.
    ///
    /// The buffer is overwritten rather than added to, so scores never leak
    /// from one generation into the next, and nothing here allocates: the
    /// score buffer and the population were sized once at build time.
    ///
    /// # Errors
    ///
    /// Returns [`SimError::UnknownStrategy`] if an agent plays a strategy the
    /// game does not define. The builder makes that unreachable, and it is
    /// reported rather than ignored because silently scoring such an agent
    /// zero would bias the run in a way no test would catch.
    ///
    /// Returns [`SimError::NonFiniteScore`] if a total overflows, which needs
    /// payoffs near `f64::MAX`. Catching it here is what lets the update
    /// below subtract two scores without ever producing a NaN.
    fn play_matches(&mut self) -> Result<(), SimError> {
        // Destructured so the borrow checker sees the game, the population,
        // the scores and the generator as four disjoint borrows rather than
        // one borrow of `self`.
        let Self {
            game,
            rng,
            strategies,
            scores,
            matches_per_agent,
            ..
        } = self;
        let population = strategies.len();
        let others = population - 1;

        for (agent, (&focal, score)) in
            strategies.iter().zip(scores.iter_mut()).enumerate()
        {
            let row = game.row(focal).ok_or(SimError::UnknownStrategy {
                strategy: focal.index(),
                strategy_count: game.strategy_count(),
            })?;

            let mut total = 0.0;
            for _ in 0..matches_per_agent.get() {
                // One draw over the other agents, mapped past the gap left by
                // the agent itself. Drawing over the whole population and
                // rejecting self-matches would spend a variable amount of
                // entropy per match and make replays depend on how often that
                // happened.
                let drawn =
                    rng.next_index(others).ok_or(SimError::PopulationTooSmall {
                        found: population,
                        minimum: WellMixed::MIN_POPULATION,
                    })?;
                let opponent_index = if drawn >= agent { drawn + 1 } else { drawn };
                // `drawn < population - 1`, so the shift lands inside the
                // population and skips `agent` exactly.
                let opponent = strategies[opponent_index];
                total += row.get(opponent.index()).copied().ok_or(
                    SimError::UnknownStrategy {
                        strategy: opponent.index(),
                        strategy_count: game.strategy_count(),
                    },
                )?;
            }
            if !total.is_finite() {
                return Err(SimError::NonFiniteScore {
                    agent,
                    score: total,
                });
            }
            *score = total;
        }

        Ok(())
    }

    /// Applies one round of Fermi pairwise comparison to the whole
    /// population.
    ///
    /// Each agent, in index order, draws one other agent as a model and
    /// adopts its strategy with probability `1 / (1 + exp(-beta * dPi))`,
    /// where `dPi` is the model's score minus its own. A better-scoring model
    /// is copied more often than a worse one, but never with certainty:
    /// selection here is a bias, not a rule, which is what keeps a finite
    /// population wandering around its equilibrium instead of locking onto
    /// it.
    ///
    /// The update is synchronous. Every decision reads the population as it
    /// was at the start of the round and writes into the second buffer, which
    /// is then swapped in. Updating in place would let a strategy copied
    /// early in the sweep be copied again later in the same round, so the
    /// outcome would depend on the order agents happen to sit in the vector.
    ///
    /// Exactly two draws are spent per agent, a model and a coin, whatever
    /// the outcome. Skipping the coin when the model plays the same strategy
    /// would be free and correct, and would also make the entropy a run
    /// consumes depend on its own state, which no replay could follow.
    fn imitate(&mut self) {
        let Self {
            rng,
            strategies,
            next_strategies,
            scores,
            selection_strength,
            ..
        } = self;
        let others = strategies.len() - 1;

        for (agent, next) in next_strategies.iter_mut().enumerate() {
            let focal = strategies[agent];
            // Same shift as the matching pass: one draw over the other
            // agents, mapped past the gap the agent itself leaves.
            let model_index = match rng.next_index(others) {
                Some(drawn) if drawn >= agent => drawn + 1,
                Some(drawn) => drawn,
                // Unreachable: the builder rejects a population below two, so
                // there is always another agent to compare against. Keeping
                // the agent put is the one choice that cannot invent a
                // strategy or bias a strategy's share.
                None => {
                    *next = focal;
                    continue;
                }
            };

            let gap = scores[model_index] - scores[agent];
            let adopt = rng.next_unit() < fermi(*selection_strength, gap);
            *next = if adopt {
                strategies[model_index]
            } else {
                focal
            };
        }

        // O(1): the two buffers trade places, so neither is ever reallocated
        // and last generation's population becomes next generation's target.
        std::mem::swap(&mut self.strategies, &mut self.next_strategies);
    }

    /// Runs one generation: every agent plays its matches, then the whole
    /// population updates at once.
    ///
    /// # Errors
    ///
    /// Propagates any error from the matching pass. The population is left
    /// untouched if one occurs, since the update never runs.
    pub fn step(&mut self) -> Result<(), SimError> {
        self.play_matches()?;
        self.imitate();
        self.generation = self.generation.next();
        self.record_shares()
    }

    /// Returns the strategy shares of every generation so far, flat and
    /// generation-major: generation `g`'s shares are the `strategy_count`
    /// entries starting at `g * strategy_count`.
    ///
    /// One flat buffer rather than a vector of rows because this crosses to
    /// JS as a single `Float64Array` view. A nested shape would mean an
    /// object per generation and a copy per frame; a strategy-major layout
    /// would mean the UI could not append a generation without moving
    /// everything after it.
    ///
    /// The replicator trajectory in Issue 2a uses this exact layout, so the
    /// two can be plotted against each other without reshaping either.
    ///
    /// It always holds at least generation 0, which is recorded at build
    /// time, and it grows by one row per [`step`]. Growth is amortised the
    /// way a `Vec` grows, so a step allocates only occasionally, and never
    /// touches the buffers the matching and update passes use.
    ///
    /// [`step`]: WellMixed::step
    pub fn share_history(&self) -> &[f64] {
        &self.history
    }

    /// Returns how many generations are recorded, which is one more than the
    /// number of steps run.
    pub fn recorded_generations(&self) -> usize {
        // Exact: the history only ever grows by whole rows.
        self.history.len() / self.game.strategy_count()
    }

    /// Returns one generation's shares, or `None` if that generation has not
    /// run yet.
    pub fn shares_at(&self, generation: Generation) -> Option<&[f64]> {
        let strategy_count = self.game.strategy_count();
        let base = generation.index().checked_mul(strategy_count)?;
        self.history.get(base..base.checked_add(strategy_count)?)
    }

    /// Returns the shares of the current generation.
    ///
    /// Always `strategy_count` entries: generation 0 is recorded before the
    /// first step, so there is never a moment with no current shares.
    pub fn current_shares(&self) -> &[f64] {
        self.history
            .rchunks_exact(self.game.strategy_count())
            .next()
            .unwrap_or(&[])
    }

    /// Appends the current population's shares to the history.
    ///
    /// Counts are accumulated directly into the new row rather than into a
    /// scratch buffer that would then be copied, and the row is divided
    /// through once at the end.
    ///
    /// # Errors
    ///
    /// Returns [`SimError::UnknownStrategy`] if an agent plays a strategy the
    /// game does not define. Checked before the row is appended, so a failed
    /// recording cannot leave a half-written generation in the history.
    fn record_shares(&mut self) -> Result<(), SimError> {
        let Self {
            game,
            strategies,
            history,
            ..
        } = self;
        let strategy_count = game.strategy_count();

        if let Some(unknown) = strategies.iter().find(|&&s| !game.contains(s)) {
            return Err(SimError::UnknownStrategy {
                strategy: unknown.index(),
                strategy_count,
            });
        }

        let base = history.len();
        history.resize(base + strategy_count, 0.0);
        let row = &mut history[base..];
        for &strategy in strategies.iter() {
            // In range: every strategy was checked against the game above.
            row[strategy.index()] += 1.0;
        }

        let population = strategies.len() as f64;
        for share in row.iter_mut() {
            *share /= population;
        }
        Ok(())
    }
}

/// Probability of adopting a model's strategy under Fermi pairwise
/// comparison: `1 / (1 + exp(-beta * gap))`.
///
/// Evaluated in whichever of the two algebraically identical forms keeps the
/// exponent negative, so `exp` is only ever called on a value in `(-inf, 0]`
/// and returns something in `(0, 1]`. Written directly, an overwhelming gap
/// would evaluate `exp` to infinity on the way to an answer that is simply 0
/// or 1, and an infinity that meets another infinity downstream becomes a
/// NaN, which compares false against every draw and would quietly freeze the
/// population forever.
fn fermi(selection_strength: f64, gap: f64) -> f64 {
    let exponent = selection_strength * gap;
    if exponent >= 0.0 {
        1.0 / (1.0 + (-exponent).exp())
    } else {
        let weight = exponent.exp();
        weight / (1.0 + weight)
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
    fn rejects_a_generation_in_which_nobody_plays() {
        let err = WellMixedBuilder::new(hawk_dove(), 100)
            .matches_per_agent(0)
            .build()
            .expect_err("a generation with no matches carries no information");
        assert!(matches!(err, SimError::NoMatches), "{err}");
    }

    #[test]
    fn a_uniform_population_scores_the_same_payoff_every_match() {
        // Every opponent plays dove, so each of the 8 matches pays exactly
        // D vs D whoever is drawn. The total is then independent of the RNG,
        // which is what makes this an exact assertion rather than a range.
        let mut sim = WellMixedBuilder::new(hawk_dove(), 64)
            .initial_shares(vec![0.0, 1.0])
            .matches_per_agent(8)
            .build()
            .expect("valid configuration");
        let dove_vs_dove = sim
            .game()
            .payoff(HawkDove::DOVE, HawkDove::DOVE)
            .expect("dove is in the game");

        sim.play_matches().expect("population plays its own game");

        assert!(
            sim.scores.iter().all(|&s| s == 8.0 * dove_vs_dove),
            "every agent should score 8 x {dove_vs_dove}, got {:?}",
            sim.scores
        );
    }

    #[test]
    fn an_agent_never_meets_itself() {
        // Two agents, one of each strategy. A hawk that could draw itself
        // would sometimes collect (V-C)/2 = -1 instead of V = 2, so over 200
        // matches an exact total of 400 is only reachable if self-matching is
        // impossible rather than merely unlikely.
        let mut sim = WellMixedBuilder::new(hawk_dove(), 2)
            .initial_shares(vec![0.5, 0.5])
            .matches_per_agent(200)
            .build()
            .expect("valid configuration");

        sim.play_matches().expect("population plays its own game");

        assert_eq!(sim.strategies(), [HawkDove::HAWK, HawkDove::DOVE]);
        assert_eq!(sim.scores[0], 200.0 * 2.0, "hawk meets only the dove");
        assert_eq!(sim.scores[1], 0.0, "dove meets only the hawk");
    }

    #[test]
    fn a_lone_hawk_out_scores_every_dove_it_exploits() {
        // The hawk meets only doves and takes V each time; a dove collects at
        // most V/2 per match. The gap is what the imitation update in 1.9
        // acts on, and it also pins the row orientation: a transposed matrix
        // would reverse this.
        let mut sim = WellMixedBuilder::new(hawk_dove(), 100)
            .initial_shares(vec![0.01, 0.99])
            .matches_per_agent(16)
            .build()
            .expect("valid configuration");

        sim.play_matches().expect("population plays its own game");

        let hawk_score = sim.scores[0];
        assert_eq!(hawk_score, 16.0 * 2.0);
        assert!(
            sim.scores[1..].iter().all(|&s| s < hawk_score),
            "the lone hawk should out-score every dove"
        );
    }

    #[test]
    fn each_generation_replaces_the_previous_scores_rather_than_adding_to_them() {
        let mut sim = WellMixedBuilder::new(hawk_dove(), 32)
            .initial_shares(vec![0.0, 1.0])
            .matches_per_agent(4)
            .build()
            .expect("valid configuration");

        sim.play_matches().expect("first generation");
        let first: Vec<_> = sim.scores.clone();
        sim.play_matches().expect("second generation");

        assert_eq!(
            sim.scores, first,
            "scores must not accumulate across passes"
        );
    }

    #[test]
    fn matching_reuses_its_buffers_instead_of_reallocating() {
        let mut sim = WellMixedBuilder::new(hawk_dove(), 256)
            .matches_per_agent(4)
            .build()
            .expect("valid configuration");
        let scores_before = sim.scores.as_ptr();
        let strategies_before = sim.strategies.as_ptr();

        for _ in 0..16 {
            sim.play_matches().expect("population plays its own game");
        }

        assert_eq!(scores_before, sim.scores.as_ptr(), "scores buffer moved");
        assert_eq!(
            strategies_before,
            sim.strategies.as_ptr(),
            "population buffer moved"
        );
    }

    #[test]
    fn matching_spends_exactly_one_draw_per_match_in_a_fixed_order() {
        // Pins the entropy budget of a generation. If a match ever drew twice
        // (or a strategy lookup drew at all), the replay would desync here
        // rather than silently diverging deep into a run.
        let population = 40;
        let matches_per_agent = 3;
        let mut sim = WellMixedBuilder::new(hawk_dove(), population)
            .matches_per_agent(matches_per_agent)
            .seed(Seed::new(17))
            .build()
            .expect("valid configuration");
        let mut reference = Rng::from(Seed::new(17));

        sim.play_matches().expect("population plays its own game");
        for _ in 0..population * matches_per_agent {
            reference.next_index(population - 1);
        }

        assert_eq!(sim.rng.next_unit(), reference.next_unit());
    }

    #[test]
    fn the_same_seed_scores_the_same_generation() {
        let build = |seed| {
            WellMixedBuilder::new(hawk_dove(), 200)
                .initial_shares(vec![0.5, 0.5])
                .matches_per_agent(8)
                .seed(Seed::new(seed))
                .build()
                .expect("valid configuration")
        };

        let mut left = build(9);
        let mut right = build(9);
        let mut other = build(10);
        for sim in [&mut left, &mut right, &mut other] {
            sim.play_matches().expect("population plays its own game");
        }

        assert_eq!(left.scores, right.scores);
        assert_ne!(left.scores, other.scores, "a different seed must diverge");
    }

    #[test]
    fn an_even_comparison_is_a_coin_flip() {
        assert_eq!(fermi(1.0, 0.0), 0.5);
        assert_eq!(fermi(0.0, 100.0), 0.5, "no selection means no preference");
        assert_eq!(fermi(0.0, -100.0), 0.5);
    }

    #[test]
    fn a_better_model_is_likelier_to_be_copied_than_a_worse_one() {
        let beta = 1.0;
        assert!(fermi(beta, 1.0) > 0.5);
        assert!(fermi(beta, -1.0) < 0.5);
        assert!(fermi(beta, 2.0) > fermi(beta, 1.0), "monotone in the gap");
        assert!(fermi(beta, 5.0) > fermi(beta, 5.0 - 1e-9));
    }

    #[test]
    fn copying_a_model_and_being_copied_are_complementary() {
        // 1/(1+e^-x) + 1/(1+e^x) = 1 exactly, and the stable form has to
        // preserve that despite evaluating the two sides by different
        // branches.
        for gap in [0.0, 0.5, 3.0, 40.0, 1e6] {
            let sum = fermi(1.3, gap) + fermi(1.3, -gap);
            assert!((sum - 1.0).abs() < 1e-12, "gap {gap} summed to {sum}");
        }
    }

    #[test]
    fn an_overwhelming_gap_saturates_instead_of_overflowing() {
        // The direct form evaluates exp(800), which is infinity. Saturating
        // at 0 and 1 is the answer that infinity was standing in for; a NaN
        // here would silently stop every agent from ever imitating.
        for (beta, gap) in [
            (1.0, 800.0),
            (1.0, -800.0),
            (1e300, 1e300),
            (1e300, -1e300),
            (f64::MAX, f64::MAX),
        ] {
            let p = fermi(beta, gap);
            assert!(p.is_finite(), "fermi({beta}, {gap}) = {p}");
            assert!((0.0..=1.0).contains(&p), "fermi({beta}, {gap}) = {p}");
            assert_eq!(p, if gap > 0.0 { 1.0 } else { 0.0 });
        }
    }

    #[test]
    fn a_step_advances_the_generation_counter() {
        let mut sim = WellMixedBuilder::new(hawk_dove(), 50)
            .build()
            .expect("valid configuration");

        assert_eq!(sim.generation(), Generation::ZERO);
        sim.step().expect("a generation runs");
        assert_eq!(sim.generation(), Generation::ZERO.next());
        sim.step().expect("a generation runs");
        assert_eq!(sim.generation().get(), 2);
    }

    #[test]
    fn a_step_never_changes_the_population_size_or_invents_a_strategy() {
        let mut sim = WellMixedBuilder::new(hawk_dove(), 128)
            .seed(Seed::new(11))
            .build()
            .expect("valid configuration");

        for _ in 0..25 {
            sim.step().expect("a generation runs");
            assert_eq!(sim.population(), 128);
            assert!(sim.strategies().iter().all(|&s| sim.game().contains(s)));
        }
    }

    #[test]
    fn a_uniform_population_cannot_change_whatever_it_draws() {
        // Everyone plays dove and scores the same, so every comparison is a
        // coin flip - but there is nothing else to copy. Any change here
        // would mean the update invented a strategy.
        let mut sim = WellMixedBuilder::new(hawk_dove(), 64)
            .initial_shares(vec![0.0, 1.0])
            .build()
            .expect("valid configuration");

        for _ in 0..20 {
            sim.step().expect("a generation runs");
        }

        assert!(sim.strategies().iter().all(|&s| s == HawkDove::DOVE));
    }

    #[test]
    fn without_selection_a_quarter_of_the_population_turns_over() {
        // beta = 0 makes every adoption a coin flip, and half the models
        // carry the other strategy, so about N/4 agents should change. This
        // is what pins the *number* of decisions: an update that skipped
        // agents, or decided twice, would miss this band.
        let mut sim = WellMixedBuilder::new(hawk_dove(), 10_000)
            .initial_shares(vec![0.5, 0.5])
            .selection_strength(0.0)
            .seed(Seed::new(5))
            .build()
            .expect("valid configuration");
        let before = sim.strategies().to_vec();

        sim.step().expect("a generation runs");

        let changed = sim
            .strategies()
            .iter()
            .zip(before.iter())
            .filter(|(now, then)| now != then)
            .count();
        let share = changed as f64 / 10_000.0;
        assert!(
            (0.22..0.28).contains(&share),
            "expected about a quarter of agents to change, got {share}"
        );
    }

    #[test]
    fn strong_selection_spreads_the_dominant_strategy() {
        // V > C, so fighting pays and hawk beats dove against any opponent.
        // A rule that copied *worse* neighbours would drive this to zero, so
        // the direction of the comparison is what is under test here.
        let mut sim = WellMixedBuilder::new(
            Game::try_from(HawkDove { v: 6.0, c: 2.0 }).expect("valid game"),
            500,
        )
        .initial_shares(vec![0.5, 0.5])
        .selection_strength(5.0)
        .seed(Seed::new(21))
        .build()
        .expect("valid configuration");

        for _ in 0..200 {
            sim.step().expect("a generation runs");
        }

        let hawks = count_of(&sim, HawkDove::HAWK) as f64 / 500.0;
        assert!(
            hawks > 0.95,
            "hawk should take over when V > C, got {hawks}"
        );
    }

    #[test]
    fn a_step_spends_a_fixed_number_of_draws_whatever_it_decides() {
        // Matching draws one partner per match; the update draws a model and
        // a coin per agent, always, even when the outcome is already
        // determined. A short-circuit anywhere would desync this replay.
        let population = 30;
        let matches_per_agent = 2;
        let mut sim = WellMixedBuilder::new(hawk_dove(), population)
            .matches_per_agent(matches_per_agent)
            .seed(Seed::new(88))
            .build()
            .expect("valid configuration");
        let mut reference = Rng::from(Seed::new(88));

        sim.step().expect("a generation runs");

        for _ in 0..population * matches_per_agent {
            reference.next_index(population - 1);
        }
        for _ in 0..population {
            reference.next_index(population - 1);
            reference.next_unit();
        }
        assert_eq!(sim.rng.next_unit(), reference.next_unit());
    }

    #[test]
    fn the_update_reuses_its_buffers_instead_of_reallocating() {
        let mut sim = WellMixedBuilder::new(hawk_dove(), 256)
            .build()
            .expect("valid configuration");
        let mut buffers = [sim.strategies.as_ptr(), sim.next_strategies.as_ptr()];
        buffers.sort_unstable();

        for _ in 0..16 {
            sim.step().expect("a generation runs");
        }

        let mut after = [sim.strategies.as_ptr(), sim.next_strategies.as_ptr()];
        after.sort_unstable();
        assert_eq!(buffers, after, "the two population buffers must be reused");
    }

    #[test]
    fn the_same_seed_replays_a_whole_run() {
        let run = |seed| {
            let mut sim = WellMixedBuilder::new(hawk_dove(), 300)
                .initial_shares(vec![0.4, 0.6])
                .seed(Seed::new(seed))
                .build()
                .expect("valid configuration");
            for _ in 0..50 {
                sim.step().expect("a generation runs");
            }
            sim.strategies().to_vec()
        };

        assert_eq!(run(6), run(6));
        assert_ne!(run(6), run(7));
    }

    #[test]
    fn generation_zero_is_recorded_before_anything_runs() {
        // The analytic overlay in 2a starts from the initial share, so the
        // two curves have to begin at the same point rather than one
        // generation apart.
        let sim = WellMixedBuilder::new(hawk_dove(), 1_000)
            .initial_shares(vec![0.3, 0.7])
            .build()
            .expect("valid configuration");

        assert_eq!(sim.recorded_generations(), 1);
        assert_eq!(sim.share_history(), [0.3, 0.7]);
        assert_eq!(sim.current_shares(), [0.3, 0.7]);
    }

    #[test]
    fn each_step_appends_exactly_one_row() {
        let mut sim = WellMixedBuilder::new(hawk_dove(), 100)
            .build()
            .expect("valid configuration");

        for expected in 1..=10 {
            sim.step().expect("a generation runs");
            assert_eq!(sim.recorded_generations(), expected + 1);
            assert_eq!(sim.share_history().len(), (expected + 1) * 2);
        }
    }

    #[test]
    fn the_history_is_generation_major() {
        // The UI reads this buffer by stride, so row g must be the contiguous
        // run at g * strategy_count. A strategy-major layout would still have
        // the right length and the right sums, and would draw nonsense.
        let mut sim = WellMixedBuilder::new(three_strategy_game(), 90)
            .initial_shares(vec![1.0, 0.0, 0.0])
            .seed(Seed::new(4))
            .build()
            .expect("valid configuration");
        for _ in 0..5 {
            sim.step().expect("a generation runs");
        }

        let strategy_count = 3;
        for generation in 0..sim.recorded_generations() {
            let row = sim
                .shares_at(Generation::new(generation as u32))
                .expect("recorded generation");
            let base = generation * strategy_count;
            assert_eq!(row, &sim.share_history()[base..base + strategy_count]);
        }
        // Nothing can invade a payoff-free game, so the run stays put and the
        // first row is recognisable wherever it is stored.
        assert_eq!(
            sim.shares_at(Generation::ZERO),
            Some([1.0, 0.0, 0.0].as_slice())
        );
    }

    #[test]
    fn every_recorded_generation_is_a_distribution() {
        let mut sim = WellMixedBuilder::new(hawk_dove(), 777)
            .initial_shares(vec![0.5, 0.5])
            .seed(Seed::new(19))
            .build()
            .expect("valid configuration");
        for _ in 0..60 {
            sim.step().expect("a generation runs");
        }

        for (generation, row) in sim.share_history().chunks_exact(2).enumerate() {
            let sum: f64 = row.iter().sum();
            assert!(
                (sum - 1.0).abs() < 1e-12,
                "generation {generation} sums to {sum}"
            );
            assert!(row.iter().all(|&s| (0.0..=1.0).contains(&s)));
        }
    }

    #[test]
    fn the_last_row_is_the_population_as_it_stands() {
        let mut sim = WellMixedBuilder::new(hawk_dove(), 400)
            .initial_shares(vec![0.5, 0.5])
            .seed(Seed::new(23))
            .build()
            .expect("valid configuration");
        for _ in 0..30 {
            sim.step().expect("a generation runs");
        }

        let hawks = count_of(&sim, HawkDove::HAWK) as f64 / 400.0;
        assert_eq!(sim.current_shares(), [hawks, 1.0 - hawks]);
        assert_eq!(
            sim.shares_at(sim.generation()),
            Some(sim.current_shares()),
            "the current shares are the row for the current generation"
        );
    }

    #[test]
    fn a_generation_that_has_not_run_has_no_row() {
        let sim = WellMixedBuilder::new(hawk_dove(), 10)
            .build()
            .expect("valid configuration");

        assert_eq!(sim.shares_at(Generation::new(1)), None);
        assert_eq!(sim.shares_at(Generation::new(9_999)), None);
    }

    #[test]
    fn the_same_seed_replays_the_same_history() {
        let run = |seed| {
            let mut sim = WellMixedBuilder::new(hawk_dove(), 250)
                .initial_shares(vec![0.45, 0.55])
                .seed(Seed::new(seed))
                .build()
                .expect("valid configuration");
            for _ in 0..40 {
                sim.step().expect("a generation runs");
            }
            sim.share_history().to_vec()
        };

        assert_eq!(run(31), run(31));
        assert_ne!(run(31), run(32));
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
