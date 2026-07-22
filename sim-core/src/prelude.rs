//! Shared vocabulary for the simulation modules.
//!
//! Every module in this crate starts with `use crate::prelude::*;` so the
//! cross-cutting types (errors, ids, seeds, generations, shares) are named the
//! same way everywhere. Simulation modules themselves (`game`, `rng`,
//! `wellmixed`, `replicator`) are deliberately *not* re-exported here: the
//! prelude carries vocabulary, not engines, so importing it can never create a
//! cycle between two simulations.

pub use crate::error::{GameError, SimError};
pub use crate::shares::Shares;
pub use crate::types::{Generation, Seed, StrategyId};
