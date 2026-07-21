//! Domain error types for the simulation core.
//!
//! Errors are typed enums built with `thiserror`; the wasm boundary in
//! `lib.rs` is the only place that converts them into `JsError`.
//!
//! Populated by Task 1.2 (`GameError`, `SimError`).
