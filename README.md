# Galápagos

*Named after the islands where Darwin observed the variation that became the
theory of natural selection — this is a lab for watching selection find
equilibria that game theory predicts.*

A spatial evolutionary game theory lab: populations of agents play games
(Hawk–Dove, Rock–Paper–Scissors, Stag Hunt, custom), and strategies spread by
imitation, selection, and mutation — on a grid and in well-mixed populations.
Simulation core in Rust (compiled to WASM), interactive frontend in TypeScript.

## Layout

- `sim-core/` — Rust crate, all simulation logic; compiled to WASM with wasm-pack
- `web/` — Vite + TypeScript frontend; renders state and forwards controls only

## Commands

```bash
# Rust unit tests (includes theory-vs-simulation assertions)
cd sim-core && cargo test

# Rebuild the WASM package (run after any sim-core change)
cd web && pnpm run wasm      # wasm-pack build ../sim-core --target web --out-dir pkg

# Dev server
cd web && pnpm run dev       # binds 0.0.0.0 so the port can be published

# Production build (type-checks first)
cd web && pnpm run build
```

## Prerequisites

Rust (stable) with the `wasm32-unknown-unknown` target, `wasm-pack`, Node ≥ 20,
pnpm.
