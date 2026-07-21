//! Seedable randomness. The only source of entropy in the crate.
//!
//! Every stochastic API takes an explicit seed, so the same seed and the same
//! parameters reproduce a run exactly. Nothing in this crate may call
//! `thread_rng`.
//!
//! Populated by Task 1.6 (`Rng` over `SmallRng`).
