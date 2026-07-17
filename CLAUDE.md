# CLAUDE.md - Galapagos (Rust / sim-core)

This file governs Rust work in `sim-core/`. UI work in `web/` is governed by
`web/CLAUDE.md`. When writing an issue, use the file matching the layer the
issue targets.

## 1. Project context

Galapagos is an evolutionary game theory lab. `sim-core/` (Rust, compiled to
WASM with wasm-pack) is the single source of truth for all simulation logic.
`web/` (Vite + TypeScript) renders state and forwards controls, nothing more.
Product spec: [SPECS.md](SPECS.md). Task list: [ISSUES.md](ISSUES.md).

## 2. Engineering priorities

You are an **expert Rust engineer**. When tradeoffs exist, prefer in this
order: correctness > safety > reliability > clarity > maintainability >
performance > brevity.

Apply high standard to engineer excellent: lint, test failures, test flakiness.
If you see one, even if it is not caused by what you are working on right now,
still get it fixed.

---

## 3. Core Rust rules (non-negotiable)

- No `unwrap()`, `expect()`, `panic!()` in production paths (tests may use them).
- Do not ignore `Result`.
- Do not silently swallow errors or invent fake APIs/crates.
- Use `Result` for recoverable errors; `Option` only when "missing" is not an error.
- Use `thiserror` for domain error types in `sim-core`.
- Use `?` for propagation and add context at boundary layers. Validate inputs
  at the boundary and fail early; preserve the source error when wrapping.
- Prefer enums over booleans for multi-state behavior.
- Document public APIs with rustdoc; for non-obvious code, document why the
  design exists, not what the code does.
- Avoid `unsafe`. If absolutely needed, keep it tiny and document `# Safety`.
- Every stochastic API takes an explicit seed; same seed + same parameters must
  reproduce the same run, every time.
- use macros if code repeats itself.
- Instead of calling structs with `std::default::Default` use `std::convert::From`
  and `std::convert::TryFrom` to convert between types, use `std::str::FromStr`
  to parse user defined types from a string.
- use cargo watch to gain testing time, i'll test with
  `cargo watch -q -c -w sim-core/src -x "test -q"`
- go for code that works => make it right => make it faster.
- Be mindful of silent memory leaks if you use smart pointers that are not
  checked at compile time (including `Closure` handles at the wasm-bindgen boundary).
- each file should do one type of action, use prelude.rs to put helpers and crates.
- use Type-driven design
- Verification pipeline, ran after each issue:
  - `cargo fmt --check` (rustfmt.toml at the root with rules)
  - `cargo clippy -- -D warnings` (watch for: unwrap, array instead of vecs,
    iter instead of loop, needless clones)
  - `cargo test`
  - `pnpm run wasm` from `web/` must build cleanly
  - rust-toolchain.toml so other machines use same toolchain
- Be mindful of not allocating memory you don't need, use borrow instead of clone.
  Hot per-generation loops (large grids) must not allocate per step.
- Avoid repeating work, like performing a hashmap lookup multiple times
- use swap_remove when you need O(1) removal from a Vec and when order does not matter

---

## 4. Design patterns and architecture

Apply these consistently:

- **Layering**
  - `sim-core` exposes pure domain logic and types; keep wasm-bindgen exports a
    thin layer over them.
  - Expose state to JS as flat buffers (`Float64Array` histories, `&[u8]` grid
    views), not nested objects; do not call back into JS from hot loops.
  - `web/` consumes those buffers; it never re-implements simulation logic.

- **Error design**
  - In `sim-core`: define clear error enums with `thiserror`. Avoid stringly
    typed errors.
  - At the wasm-bindgen boundary: convert typed errors to `JsError` with context.
  - Do not hide the underlying cause; wrap it.

- **Type design**
  - Prefer strong, domain-specific types over bare `String`/`i64`.
  - Use newtypes for strategy ids, seeds, generations when it improves clarity.
  - Make invalid states unrepresentable where practical.

- **Patterns**
  - Use **builder** pattern for configs/structs with many fields or validation
    (simulation parameter sets are the main case here).
  - Use **factory/trait objects** when runtime selection between implementations
    is needed (e.g. update rules: imitation, Fermi, Moran).
  - Use **RAII** for resources and cleanup.
  - Consider **typestate** for clear state transitions (e.g. configured vs
    running simulation).
    
---

## 5. Testing and workflow (TDD)

- For each task/issue:
  - Add or update tests **before** implementation.
  - Then implement code to satisfy those tests.
  - Run `cargo test` and ensure green before moving on.
- Where theory makes a quantitative claim, it gets a `cargo test` assertion with
  an explicit tolerance and a fixed seed (e.g. Hawk-Dove converging to V/C),
  not just a visual check.
- Test failure paths, not just success paths. Every fixed bug gets a
  regression test.
- Use property-based testing where invariants benefit from it (e.g. strategy
  shares sum to 1, same seed reproduces the same run, ODE invariants).
- After implementation:
  - Run `cargo fmt --check`.
- Keep changes small:
  - One issue = one cohesive change set; hard stop after each issue for review
    (see ISSUES.md cadence).
  - Do not modify unrelated modules in the same patch.
- Preserve existing tests; if behavior must change, update tests and clearly
  reflect the new contract.

---

## 6. What not to do

- Do not:
  - use em dash, use plain dash "-" instead.
  - when making technical decisions, do not give much weight to development
    cost. Instead prefer quality, simplicity, robustness, scalability and long
    term maintainability.
  - Re-implement `sim-core` logic in the frontend.
  - Add new dependencies without a clear, brief justification.
  - Introduce panics or unchecked failures.
  - Combine multiple unrelated tasks in one change.
  - git add, commit, push, rebase
  - finish a task without running the verification pipeline (section 3).

If anything in the current task seems ambiguous or conflicts with these rules,
**ask a concrete question** instead of guessing.
