/**
 * The parameters a Hawk-Dove run takes.
 *
 * Named and typed on the UI's terms; the conversion to what wasm-bindgen
 * expects (a `bigint` seed, `undefined` for "use the documented default")
 * happens once, in the wrappers.
 *
 * Nothing here is range-checked. `sim-core` validates every one of these and
 * says which value it rejected and why, and duplicating those rules in
 * TypeScript would create a second, quietly diverging source of truth. The
 * sliders in 2b.8 are built so they cannot express a rejected configuration in
 * the first place.
 */
export interface HawkDoveParams {
  /** Resource value `V`. */
  readonly v: number;
  /** Fight cost `C`. Below `V` there is an interior equilibrium at `V/C`. */
  readonly c: number;
  /** Number of agents. */
  readonly population: number;
  /** Share of the population that starts as hawks, in `[0, 1]`. */
  readonly initialHawkShare: number;
  /**
   * Seed for the run's RNG. The same seed and parameters reproduce the same
   * run exactly; it crosses to WASM as a 64-bit value.
   */
  readonly seed: number;
  /** Selection strength beta. Omitted means the core's documented default. */
  readonly selectionStrength?: number;
  /** Matches each agent plays per generation. Omitted means the default. */
  readonly matchesPerAgent?: number;
}

/**
 * The parameters the analytic trajectory takes: exactly the subset of the
 * run's that it shares.
 *
 * Derived with `Pick` rather than written out, so the overlay cannot drift
 * onto a different `V`, `C` or starting point from the run it is drawn
 * against - the whole point of showing them on one chart.
 */
export type TrajectoryParams = Pick<
  HawkDoveParams,
  "v" | "c" | "initialHawkShare"
>;
