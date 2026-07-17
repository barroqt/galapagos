# Galápagos - Development Issues

*Chronological, buildable tasks. Each issue ships a playable feature **and**
teaches one game-theory concept, and each builds on the previous. For the
timeless description of the product these issues assemble, see
[SPECS.md](SPECS.md).*

**Cadence - hard stop after every issue.** Development pauses at the end of each
issue for review and explicit go-ahead before the next begins. Claude writes the
code and explains each theory/design decision; the reviewer reads closely and
questions. Split pairs (Na/Nb) are two separate issues with two separate stops.

**Tags - one session, one layer.**
- **[RUST]** - touches `sim-core/` only. Run from the **repo root**, governed by
  the root `CLAUDE.md`. Never edits `web/`.
- **[UI]** - touches `web/` only. Run from a session **inside `web/`**, governed
  by `web/CLAUDE.md`. May rebuild the WASM package (`pnpm run wasm`) but never
  edits Rust source.
- In a split pair, the [RUST] half lands first; the [UI] half starts by running
  `pnpm run wasm` to pick it up.

Conventions used throughout:
- All simulation logic lives in `sim-core` (Rust → WASM). The web app renders and
  controls, nothing more.
- The frontend uses **pnpm** (never npm/yarn).
- Every stochastic run takes an explicit **seed** so results are reproducible.
- Claims made by theory get a `cargo test` assertion where possible, not just a
  visual check.
- The web app is **desktop-only**, **dark-themed with warm naturalist accents**,
  **minimal-motion** (only sims/charts animate), and **interactive-first**
  (minimal prose). Each module opens on a curated default and discloses expert
  depth on demand.

---

## Issue 0 - Baseline (already in place)

The scaffold exists: a Rust → WASM → TypeScript pipeline proven end to end by a
placeholder `Sim` counter (`sim-core/src/lib.rs`, `web/src/main.ts`), a Vite +
TypeScript web app (pnpm), and the wasm-pack build wired up. Issue 1 replaces
the placeholder with real simulation modules; the UI shell lands in Issue 2b.

---

## Issue 1 [RUST] - Hawk–Dove game + well-mixed population simulation

**Game theory concepts:** payoff matrix, expected payoff/fitness, the Hawk–Dove
game, why a mixed Nash equilibrium becomes a stable population *share*.

**Scope (`sim-core` only, no UI):**
- `game.rs`: a `Game` type holding an N×N payoff matrix; constructor for
  Hawk–Dove parameterized by resource value `V` and fight cost `C` (payoffs:
  H vs H = (V−C)/2, H vs D = V, D vs H = 0, D vs D = V/2).
- `wellmixed.rs`: agent-based well-mixed population - N agents with pure
  strategies; each generation, random pairwise matches accumulate payoffs, then a
  stochastic imitation update (e.g. pairwise comparison / Moran-style: copy a
  random other agent with probability proportional to payoff difference).
  Seedable RNG (`rng.rs`).
- History recording: strategy shares per generation, exposed to JS as
  `Float64Array`.
- **Test:** for V=2, C=4, the long-run hawk share converges to V/C = 0.5 within
  tolerance.

**Done when:** `cargo test` passes, including the V/C convergence test.

---

## Issue 2a [RUST] - Replicator dynamics ODE

**Game theory concepts:** the replicator equation, deterministic vs. stochastic
dynamics, finite-population noise.

**Scope (`sim-core` only):**
- RK4 integrator for the 2-strategy replicator ODE
  `ẋ = x(1−x)(f_H(x) − f_D(x))`; exposed alongside the agent-based sim.
- Trajectory exposed to JS as a `Float64Array` (same shape as the agent-based
  share history, so the UI can overlay them directly).
- **Test:** for V<C, the ODE converges to the interior equilibrium V/C from any
  interior initial condition, within integration tolerance.

**Done when:** `cargo test` passes, including the ODE equilibrium test.

---

## Issue 2b [UI] - First UI: hub shell + share chart with analytic overlay

**Game theory concepts:** deterministic vs. stochastic dynamics made visible,
basins of attraction.

**Scope (`web` only):**
- Stand up the **hub-of-modules shell** (dark, naturalist-accented, desktop
  layout) with the first real module - Hawk–Dove. Run the agent-based sim live,
  plot hawk share over time on a chart, and overlay the analytic ODE trajectory
  from the same initial condition.
- Controls follow **progressive disclosure**: the module opens on a sensible
  curated default; a disclosed panel exposes full parameter control - sliders for
  V and C, population size N, initial hawk share, seed - plus play/pause/reset.
- **Live numeric readouts:** current hawk/dove share and the analytic equilibrium
  value shown precisely alongside the chart.

**Done when:** moving the V/C sliders visibly moves the convergence level, the ODE
overlay tracks the simulation, and the module lives in the hub with the default
scenario self-explanatory on open.

---

## Issue 3a [RUST] - Spatial grid (toroidal lattice) with imitation dynamics

**Game theory concepts:** local vs. global interaction, network reciprocity,
cluster formation, why spatial outcomes deviate from well-mixed predictions.

**Scope (`sim-core` only):**
- `spatial.rs`: toroidal W×H grid of pure strategies; each step every cell plays
  its 8 neighbors, then adopts the strategy of its best-scoring neighbor
  (unconditional imitation), with an optional noise/error rate.
- Grid state exposed as a zero-copy `&[u8]` view for rendering; spatial strategy
  shares recorded per step (for the side-by-side chart in 3b).
- A setter to write a strategy into an individual cell (or small brush region),
  so the UI can implement click-to-paint without touching sim logic.
- **Test:** identical seed + parameters reproduce the identical grid state after
  K steps; for some (V, C) range the spatial hawk share deviates from the
  well-mixed V/C prediction beyond noise.

**Done when:** `cargo test` passes, including reproducibility and
spatial-deviation tests.

---

## Issue 3b [UI] - WebGL grid renderer + side-by-side mode

**Game theory concepts:** watching network reciprocity and cluster formation.

**Scope (`web` only):**
- Introduce the **WebGL grid renderer** here (not deferred) - must handle
  512×512 at interactive speed and render lushly (glowing cells, clean color),
  reading the grid through the zero-copy `&[u8]` view.
- Speed control (steps per frame), pause/step-once, seed control, and
  click-to-paint strategies on the grid (via the 3a cell setter).
- Side-by-side mode: spatial run and well-mixed run with identical (V, C) - the
  share chart shows both, with **live numeric readouts** for each.

**Done when:** clusters visibly form, the WebGL renderer holds interactive speed
at 512×512, and the spatial hawk share measurably differs from the well-mixed V/C
prediction for some parameter range.

---

## Issue 4a [RUST] - Generalize to 3+ strategies: Rock–Paper–Scissors

**Game theory concepts:** cyclic dominance, games with no ESS, interior fixed
points, orbits.

**Scope (`sim-core` only):**
- RPS payoff matrix (win 1, lose −1, tie 0, optionally a win/loss asymmetry
  parameter).
- Generalize the well-mixed sim, replicator ODE (RK4 on the simplex), and
  spatial sim to 3+ strategies; histories and grid views generalize accordingly.
- **Test:** the well-mixed replicator ODE conserves its known invariant (orbits
  around (⅓,⅓,⅓) don't spiral in/out beyond integration error).

**Done when:** `cargo test` passes, including the invariant-conservation test.

---

## Issue 4b [UI] - Ternary simplex plot + spiral waves

**Game theory concepts:** orbits on the simplex, how spatial structure
stabilizes biodiversity.

**Scope (`web` only):**
- **Ternary simplex plot** - the live population trajectory drawn inside a
  triangle (the classic textbook figure, animated).
- WebGL 3-color grid rendering for the spatial version.
- Minimal in-context framing: why no strategy can be evolutionarily stable here,
  what the orbits mean, why the spatial version forms spiral waves and keeps all
  three alive.

**Done when:** simplex orbits are visible in well-mixed mode and spiral waves
appear on a 256×256 grid with no extinction over long runs.

---

## Issue 5a [RUST] - Update-rule zoo + invasion experiments

**Game theory concepts:** the formal definition of ESS (a resident strategy that
cannot be invaded), how conclusions depend on the update/selection rule.

**Scope (`sim-core` only):**
- Selectable update rules for both well-mixed and spatial sims - unconditional
  imitation, pairwise comparison (Fermi function with selection strength β),
  birth–death / Moran - behind a common interface the UI can enumerate.
- Invasion experiment mode: initialize a resident population, drop in a mutant
  cluster (spatial) or mutant fraction (well-mixed), run R replicates, report
  invasion probability.
- **Test:** in Hawk–Dove with V<C, a pure-Dove resident is invadable by Hawks, and
  the mixed V/C state resists invasion by both pure strategies.

**Done when:** invasion probabilities match ESS predictions in the tested cases.

---

## Issue 5b [UI] - Batch / experiment runner

**Game theory concepts:** ESS made operational - measuring invasion probability.

**Scope (`web` only):**
- UI for choosing the update rule (each rule labelled and briefly documented
  in-UI) for both well-mixed and spatial modules.
- The **batch / experiment runner** lands here - UI for configuring and launching
  invasion batches, with aggregated results as a table / bar chart.

**Done when:** an invasion batch can be configured and launched entirely from the
UI and its aggregated results reproduce the 5a test cases.

---

## Issue 6a [RUST] - Continuous strategies + mutation (adaptive dynamics) + Stag Hunt

**Game theory concepts:** strategy spaces, mutation–selection balance, evolution
*finding* an equilibrium nobody computed, adaptive dynamics.

**Scope (`sim-core` only):**
- Agents carry a continuous trait p ∈ [0,1] (probability of playing Hawk);
  reproduction copies with Gaussian mutation (σ configurable); works in both
  well-mixed and spatial modes.
- Trait history exposed to JS as **binned counts per generation**
  (generation × trait bins), ready for heatmap rendering - the UI never receives
  raw per-agent trait arrays.
- Add **Stag Hunt** as a second built-in game (two pure equilibria,
  risk-dominance vs. payoff-dominance).
- **Test:** the long-run trait distribution concentrates around the analytic ESS
  value V/C within tolerance.

**Done when:** `cargo test` passes, including the trait-concentration test.

---

## Issue 6b [UI] - Trait-distribution heatmap + Stag Hunt module

**Game theory concepts:** watching evolution climb toward an equilibrium;
equilibrium selection in Stag Hunt.

**Scope (`web` only):**
- **Trait-distribution heatmap** over time (generation × trait bins) rendered
  from the 6a binned data, with the analytic ESS value V/C marked as a line -
  the distribution should climb toward it and hover around it.
- Stag Hunt module: which equilibrium does evolution select, and how do mutation
  rate / spatial structure change that?

**Done when:** the trait distribution demonstrably concentrates around V/C, and
Stag Hunt equilibrium selection responds to parameters.

---

## Issue 7 [UI] - Sandbox module + presets

**Game theory concepts:** none new - this issue turns the tool into an open lab
for self-directed experiments (public goods games, custom dilemmas, invasion
tournaments).

**Scope (`web` only):**
- A fully editable payoff matrix (2–4 strategies with names and colors); all
  parameters from previous issues exposed in one sandbox module, with the same
  progressive-disclosure and batch-runner affordances. Strategy names and colors
  are presentation-only and stay in the UI; `sim-core` sees only the matrix.
- Preset library: every game from Issues 1–6 plus Public Goods (with and without
  punishment) as loadable presets, surfaced as curated entry points in the hub.
- Polish pass: hub/module layout, keyboard shortcuts (space = pause, → = step),
  and a performance check at 512×512.
- *Nice-to-have (optional):* URL-encoded scenarios (parameters + seed) so a pasted
  link reproduces an exact run, with a "Copy link" button. Seed-based
  reproducibility is required regardless; public link-sharing is not a headline.
- If a missing `sim-core` capability surfaces (e.g. a matrix shape the sims
  don't accept), stop and file it as a minimal separate [RUST] issue first - do
  not work around it in TypeScript.

**Done when:** a custom 4-strategy game can be defined and run entirely from the
UI, and the preset library covers Issues 1–6 plus Public Goods.

---

## Backlog (not scheduled - reconsider after Issue 7)

- Reinforcement-learning agents (Q-learning / bandits) as an alternative to
  imitation dynamics. [RUST]
- Interaction networks beyond the lattice (small-world, scale-free) - does
  reciprocity survive? [RUST]
- Asymmetric games (owner/intruder Hawk–Dove → the "Bourgeois" strategy). [RUST]
- WebGL renderer for very large grids (2048×2048). [UI]
- Recording / export: PNG snapshots, CSV of share histories. [UI]
- Public URL-encoded scenario sharing (if not already done as the Issue 7
  nice-to-have). [UI]
