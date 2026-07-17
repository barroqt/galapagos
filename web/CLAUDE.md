# CLAUDE.md - Galapagos (web UI)

This file governs UI work in `web/`. Rust work in `sim-core/` is governed by
the root `CLAUDE.md`. When writing an issue, use the file matching the layer
the issue targets.

## 1. Project context

`web/` is a Vite + TypeScript frontend. It renders simulation state and
forwards controls; **all simulation logic lives in `sim-core` (WASM)** and is
never re-implemented here. Product spec: [../SPECS.md](../SPECS.md), especially
sections 3 (design principles), 4 (experience) and 8 (visual spec).

## 2. Core rules

- Use **pnpm** (never npm/yarn): `pnpm run dev`, `pnpm run build`, `pnpm run wasm`.
- Strict TypeScript, no `any`. `pnpm run build` (which type-checks) must pass.
- After any `sim-core` change, rebuild the WASM package with `pnpm run wasm`
  before testing the UI.
- Consume WASM state through typed-array views (`Float64Array`, `Uint8Array`);
  do not copy buffers per frame, and do not leak wasm-bindgen objects deep into
  UI code - keep them at the boundary.
- Desktop-only. No mobile layouts, no responsive effort.
- Performance is a feature: WebGL for population/grid renderers, 512x512 grids
  stay interactive, no per-frame allocations in render loops.

## 3. Look and feel (from SPECS.md)

- Dark, rich base; simulations and charts glow against it. Accents are warm and
  naturalist. No pure black/white; stay on the established palette.
- Minimal motion: only simulations and live charts animate. UI chrome is snappy
  and static, no decorative transitions.
- Interactive-first: minimal prose, strong labels and tooltips.
- Every module opens on a curated default that demonstrates its concept
  immediately; expert depth (full parameters, live numeric readouts, batch
  runner) is behind progressive disclosure, never in the way.
- Be obsessed with pixel perfection. If something clearly looks off, even if
  unrelated to the current task, fix it along the way.

## 4. Workflow

- Verify visually in `pnpm run dev` after every UI change; be picky about what
  you see.
- One issue = one cohesive change set; hard stop after each issue for review
  (see ../ISSUES.md cadence).

## 5. What not to do

- Do not:
  - use em dash, use plain dash "-" instead.
  - put simulation logic, payoff math, or update rules in TypeScript.
  - add dependencies without a brief justification (best-in-class tooling is
    welcome per SPECS.md, but each pick is justified).
  - combine multiple unrelated tasks in one change.
  - git add, commit, push, rebase.
  - finish a task without `pnpm run build` passing.

If anything in the current task seems ambiguous or conflicts with these rules,
**ask a concrete question** instead of guessing.
