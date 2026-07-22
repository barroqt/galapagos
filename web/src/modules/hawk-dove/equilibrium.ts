/**
 * What Hawk-Dove theory predicts, for display beside the run.
 *
 * # Why this is in TypeScript
 *
 * The rule for this layer is that payoff math lives in `sim-core`, and this is
 * the one documented exception: a closed form used to *label* a chart, never
 * to drive anything. Nothing here feeds a simulation, and no simulation reads
 * it - delete this file and every run behaves identically.
 *
 * It survives only as long as the closed form does. Issue 7 brings custom
 * payoff matrices of 2 to 4 strategies, which have no formula to retype, so
 * the equilibrium has to be computed by `sim-core` and exposed for display at
 * that point. This file is deleted then rather than generalised.
 */

/** Where a Hawk-Dove population settles, and which kind of rest point it is. */
export interface Equilibrium {
  /** Share of hawks at equilibrium, in `[0, 1]`. */
  readonly hawkShare: number;
  /**
   * `mixed` - an interior rest point at `V/C` that both strategies survive at.
   * `hawk` - no interior rest point: hawks dominate and take the population.
   */
  readonly kind: "mixed" | "hawk";
}

/**
 * The equilibrium hawk share for resource value `v` and fight cost `c`.
 *
 * With `V < C`, a hawk meeting a hawk loses more than the resource is worth,
 * so hawks are only worth playing when they are rare: the population settles
 * at the mixed equilibrium `V/C`, which is also the ESS.
 *
 * With `V >= C`, fighting is never worse than conceding, hawk weakly dominates
 * dove, and the interior rest point leaves the simplex - `V/C` would be 1 or
 * more, which is not a share of a population that also contains doves. The
 * equilibrium is then the boundary: all hawks. That is what "no interior
 * equilibrium" means on screen, and it is why the readout names the kind
 * rather than only printing a number.
 */
export function hawkDoveEquilibrium(v: number, c: number): Equilibrium {
  if (v < c) {
    return { hawkShare: v / c, kind: "mixed" };
  }
  return { hawkShare: 1, kind: "hawk" };
}
