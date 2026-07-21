//! Shared vocabulary for the simulation modules.
//!
//! Every module in this crate starts with `use crate::prelude::*;` so the
//! cross-cutting types (errors, ids, seeds, generations) are named the same
//! way everywhere. Simulation modules themselves (`game`, `rng`, `wellmixed`)
//! are deliberately *not* re-exported here: the prelude carries vocabulary,
//! not engines, so importing it can never create a cycle between two
//! simulations.
//!
//! The re-exports land with the types themselves - `crate::types::*` in Task
//! 1.3. Adding one while its module is empty would only be a glob over nothing
//! that `unused_imports` rejects.

pub use crate::error::{GameError, SimError};
