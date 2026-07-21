//! Domain newtypes shared across the simulation modules.
//!
//! Each of these wraps a single integer whose meaning would otherwise be
//! carried by a comment. A `usize` that is really a strategy index and a
//! `usize` that is really an agent index are interchangeable to the compiler
//! and never interchangeable to the simulation, so they get distinct types.
//!
//! The wrapped value is private throughout: constructing one goes through
//! `From` or `new`, and reading it back goes through `get`, `index` or a
//! `From` impl in the other direction. Arithmetic is deliberately *not*
//! implemented - adding two strategy ids is meaningless, and the one counter
//! that does advance (`Generation`) does so through a named method.

use std::fmt;

/// Which pure strategy an agent or cell plays, as an index into the payoff
/// matrix.
///
/// The representation is `u8` (and `#[repr(transparent)]`) because Issue 3a
/// publishes the spatial grid to JS as a zero-copy `&[u8]`; a wider or
/// differently laid out id would force a per-frame conversion pass over every
/// cell. 256 distinct strategies is far beyond anything the games here need.
///
/// Ordering is by raw value, which the spatial imitation tie-break relies on.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrategyId(u8);

impl StrategyId {
    /// Wraps a raw strategy index.
    ///
    /// Whether the index actually exists in a given game depends on that
    /// game's matrix, so it is checked where the two meet, not here.
    pub const fn new(raw: u8) -> Self {
        Self(raw)
    }

    /// Returns the raw byte, as written into the grid buffer JS reads.
    pub const fn get(self) -> u8 {
        self.0
    }

    /// Returns the id as a `usize` for indexing a payoff matrix or a share
    /// buffer, without the cast cluttering the call site.
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Seed of a stochastic run.
///
/// Every stochastic API in this crate takes one explicitly: the same seed and
/// the same parameters must reproduce a run exactly, which is why nothing here
/// may reach for `thread_rng`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Seed(u64);

impl Seed {
    /// Wraps a raw seed value. Every `u64` is a valid seed.
    pub const fn new(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw seed, for handing to the generator that consumes it.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// How many update steps a simulation has taken.
///
/// Generation 0 is the initial population, recorded before the first step, so
/// a generation doubles as an index into the share history.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Generation(u32);

impl Generation {
    /// The initial population, before any update has been applied.
    pub const ZERO: Self = Self(0);

    /// Wraps a raw generation count.
    pub const fn new(raw: u32) -> Self {
        Self(raw)
    }

    /// Returns the raw count, for display and for crossing to JS.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Returns the generation as a `usize` for indexing the share history.
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Returns the following generation.
    ///
    /// Saturates at `u32::MAX` rather than panicking or wrapping. A run that
    /// long is unreachable in a browser session, but of the two non-panicking
    /// options only saturation keeps the counter monotone, and comparisons
    /// between generations stay meaningful.
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Generates the conversions and `Display` for a newtype over an integer.
///
/// Written once because all three types want the identical set and hand-rolled
/// copies drift: `From` in both directions, and a `Display` that prints the
/// bare number so it composes inside a sentence.
macro_rules! integer_newtype {
    ($name:ident, $raw:ty) => {
        impl From<$raw> for $name {
            fn from(raw: $raw) -> Self {
                Self::new(raw)
            }
        }

        impl From<$name> for $raw {
            fn from(value: $name) -> Self {
                value.get()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

integer_newtype!(StrategyId, u8);
integer_newtype!(Seed, u64);
integer_newtype!(Generation, u32);

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn strategy_id_is_one_byte_wide() {
        // Issue 3a hands the grid to JS as `&[u8]`. A grid of `StrategyId`
        // can only be that view for free if the newtype costs nothing.
        assert_eq!(size_of::<StrategyId>(), size_of::<u8>());
    }

    #[test]
    fn strategy_id_round_trips_through_every_byte() {
        for raw in u8::MIN..=u8::MAX {
            let id = StrategyId::from(raw);
            assert_eq!(u8::from(id), raw);
            assert_eq!(id.get(), raw);
            assert_eq!(id.index(), usize::from(raw));
        }
    }

    #[test]
    fn strategy_ids_order_by_their_raw_value() {
        // 3a.4 breaks imitation ties by lowest id, so the order has to be the
        // numeric one rather than whatever a derive happens to produce.
        assert!(StrategyId::from(0) < StrategyId::from(1));
        assert_eq!(
            [StrategyId::from(2), StrategyId::from(0)].iter().min(),
            Some(&StrategyId::from(0))
        );
    }

    #[test]
    fn seed_round_trips() {
        for raw in [0, 1, 42, u64::MAX] {
            let seed = Seed::from(raw);
            assert_eq!(u64::from(seed), raw);
            assert_eq!(seed.get(), raw);
        }
    }

    #[test]
    fn generation_starts_at_zero_and_counts_up() {
        assert_eq!(Generation::ZERO.get(), 0);
        assert_eq!(Generation::ZERO.index(), 0);

        let g = Generation::ZERO.next();
        assert_eq!(g.get(), 1);
        assert_eq!(g.next().get(), 2);
    }

    #[test]
    fn generation_saturates_instead_of_wrapping_or_panicking() {
        // Unreachable in a browser session, but it must stay monotone: a
        // wrapped counter would silently reorder history.
        let last = Generation::from(u32::MAX);
        assert_eq!(last.next(), last);
    }

    #[test]
    fn generation_round_trips_and_orders() {
        let g = Generation::from(7);
        assert_eq!(u32::from(g), 7);
        assert_eq!(g.index(), 7);
        assert!(Generation::ZERO < g);
    }

    #[test]
    fn display_shows_the_bare_number_for_use_inside_messages() {
        assert_eq!(StrategyId::from(3).to_string(), "3");
        assert_eq!(Seed::from(12345).to_string(), "12345");
        assert_eq!(Generation::from(9).to_string(), "9");
    }
}
