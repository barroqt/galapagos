# Galápagos — Specification

*This document describes **what Galápagos is** — its vision, principles, and the
shape of the finished product. It is timeless: it does not track build order or
progress. The chronological, buildable tasks live in [ISSUES.md](ISSUES.md).*

---

## 1. What Galápagos is

Galápagos is a desktop web lab for **evolutionary game theory**. Populations of
agents play games — Hawk–Dove, Rock–Paper–Scissors, Stag Hunt, and custom
dilemmas — and strategies spread by imitation, selection, and mutation, both in
well-mixed populations and on a spatial grid. You watch selection *find* the
equilibria that theory predicts, and you can reach in and change the rules.

The simulation core is written in Rust and compiled to WebAssembly; the frontend
is TypeScript with WebGL rendering. The name nods to the islands where Darwin
observed the variation that became the theory of natural selection.

## 2. Goals & non-goals

**Goals**
- A **portfolio-grade public artifact**: visual craft and end-product quality are
  first-class, ahead of development cost.
- **A beginner and an expert both feel at home** in the same tool.
- **Interactive-first**: understanding comes from manipulating parameters and
  watching outcomes, not from reading.
- **Correct by construction**: theory claims are backed by tests, and every
  stochastic run is reproducible from a seed.

**Non-goals**
- Mobile or small-screen support — this is unapologetically a desktop experience.
- Heavy prose or textbook-style explanation.
- A literal illustrated-island metaphor for navigation (the naturalist *mood* is
  kept; the gimmick is not).
- Data export (CSV/PNG) — not part of the current definition.
- Classical / non-evolutionary game theory.

## 3. Design principles

- **Quality over cost.** Choose the approach that yields the best end product;
  development effort is not a limiting factor. Tooling (UI framework, charting
  library, WebGL helper) is chosen best-in-class and justified per need. The
  Rust/WASM simulation core is fixed.
- **Beginner-and-expert-at-home via progressive disclosure.** Every module opens
  on a clean, sensible, curated default. Depth — full parameter control, live
  numbers, batch experiments — is revealed on demand, never in the way.
- **Dark and glowing, with naturalist warmth.** A deep, rich base lets the
  simulations and charts glow; accents are warm and naturalist (a Darwin/
  Galápagos palette).
- **Minimal motion.** UI chrome is snappy and static; only the simulations
  animate. No decorative transitions.
- **The sims teach, not the text.** Prose is minimal; strong labels and tooltips
  orient. Curated default scenarios frame each concept.

## 4. Experience & navigation

- **Hub of modules.** A home hub lists the concept modules. Newcomers get a
  clearly signposted **recommended path**, but any module can be opened in any
  order. The open **sandbox** is one destination among them.
- **Per-module anatomy.** Each module presents:
  - a **curated default view** — a good, self-explanatory starting scenario that
    demonstrates the concept the moment it opens (no onboarding or tutorial);
  - **progressively disclosed expert controls** — full parameters, live numeric
    readouts, and where relevant a batch/experiment runner, tucked behind
    disclosure until wanted.
- **Onboarding is "sensible defaults."** There is no forced tour; the design
  itself guides.

## 5. Concepts & games covered

Described as capabilities the finished tool has, not a build order.

- **Games:** Hawk–Dove (resource value `V`, fight cost `C`), Rock–Paper–Scissors
  (cyclic dominance, optional win/loss asymmetry), Stag Hunt (payoff- vs
  risk-dominance), and **custom** payoff matrices (2–4 named strategies), including
  Public Goods dilemmas.
- **Population models:**
  - **Well-mixed** agent-based populations with random pairwise matches and a
    stochastic update.
  - The **replicator dynamics ODE** (deterministic), integrated with RK4, shown
    as an analytic overlay against the agent-based run.
  - **Spatial** populations on a toroidal lattice with local interaction and
    imitation, where clusters, network reciprocity, and spiral waves emerge.
- **Update / selection rules:** unconditional imitation, pairwise comparison
  (Fermi function with selection strength β), and birth–death / Moran.
- **ESS & invasion:** initialize a resident population, introduce a mutant
  fraction (well-mixed) or cluster (spatial), run replicates, and measure
  invasion probability — the operational definition of an evolutionarily stable
  strategy.
- **Adaptive dynamics:** agents carrying a continuous trait with Gaussian
  mutation, so the population evolves toward equilibria nobody computed by hand.

## 6. Expert affordances

Required for the expert to feel at home:

- **Live numeric readouts** — current strategy shares, payoffs, and relevant
  equilibrium values shown precisely alongside the visuals.
- **Full parameter control** — every rate, strength, and size; the update rule;
  and custom payoff matrices.
- **Batch / experiment runner** — run many replicates or parameter sweeps and see
  aggregated results (e.g. invasion probabilities, phase diagrams).

## 7. Architecture

- **`sim-core/` (Rust → WASM) is the single source of truth** for all simulation
  logic. Anticipated modules: `game` (payoff matrices), `wellmixed` (agent-based
  population + replicator ODE), `spatial` (toroidal grid, local interaction,
  imitation), and `rng` (seedable RNG helpers). State is exposed to JavaScript
  efficiently (e.g. `Float64Array` histories, zero-copy `&[u8]` grid views).
- **`web/` (TypeScript + WebGL) renders state and forwards controls only.** It
  contains no simulation logic. It owns the hub, the module UIs, the WebGL
  renderers, and the charts.
- **Reproducibility.** Every stochastic run takes an explicit seed; the same seed
  and parameters reproduce the same run.
- **Testing discipline.** Where theory makes a quantitative claim, it gets a
  `cargo test` assertion (e.g. Hawk–Dove converging to the `V/C` share), not just
  a visual check.

## 8. Visual & interaction specification

- **Base:** deep/dark, rich background; simulations and charts glow against it.
- **Accents:** warm naturalist tones.
- **Rendering:** **WebGL** for the population/grid renderers from the outset, so
  large grids (target 512×512) stay smooth and the visuals can be lush.
- **Charts:** a share-over-time chart (with analytic ODE overlay), a **ternary
  simplex plot** for 3-strategy trajectories, and a **trait-distribution heatmap**
  (generation × trait bins) for adaptive dynamics.
- **Motion:** only the simulations and live charts move; UI chrome does not.

## 9. Quality bar & invariants

- Reproducible from a seed, every time.
- Theory-vs-simulation claims covered by tests.
- 512×512 spatial grids render at interactive speed.
- Desktop-grade polish throughout; a beginner is never lost and an expert is never
  limited.

## 10. Deferred / backlog

Out of the current definition; candidates for reconsideration later:

- Reinforcement-learning agents (Q-learning / bandits) as an alternative to
  imitation dynamics.
- Interaction networks beyond the lattice (small-world, scale-free).
- Asymmetric games (owner/intruder Hawk–Dove → the "Bourgeois" strategy).
- WebGL rendering for very large grids (2048×2048).
- Recording / export: PNG snapshots, CSV of share histories.
- Public URL-encoded scenario sharing (seed-based reproducibility is required;
  sharing exact runs by link is a nice-to-have, not a headline feature).
