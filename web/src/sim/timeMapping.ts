/**
 * How one generation of the agent-based run maps onto the replicator ODE's
 * continuous time.
 *
 * The two models measure time differently, so overlaying one on the other
 * needs a scale factor. Picking that factor by dragging it until the curves
 * look aligned would make the overlay a drawing rather than a prediction, so
 * it is derived here and the derivation is the documentation.
 */
import type { HawkDoveParams } from "../core";

/** The parameters the mapping depends on. Both are required: see below. */
export type TimeMappingParams = Required<
  Pick<HawkDoveParams, "selectionStrength" | "matchesPerAgent">
>;

/**
 * The ODE time step that corresponds to one generation.
 *
 * # Derivation
 *
 * In a generation, every agent draws one model at random and adopts its
 * strategy with the Fermi probability `g(z) = 1 / (1 + exp(-z))` evaluated at
 * `z = beta * (model score - own score)`. Scores are totals over
 * `matchesPerAgent` matches, so in expectation that gap is
 * `m * (f_model - f_own)`, where `f` is the per-match expected payoff the
 * replicator equation uses.
 *
 * Summing over the population, hawks are gained when a dove draws a hawk and
 * copies it, and lost the other way round, so with `dPi = f_hawk - f_dove`:
 *
 * ```text
 * E[dx] = x(1-x) * [g(beta*m*dPi) - g(-beta*m*dPi)]
 *       = x(1-x) * tanh(beta*m*dPi / 2)
 * ```
 *
 * The replicator equation is `dx/dt = x(1-x) * dPi`. For small argument
 * `tanh(z/2) ~ z/2`, so one generation advances the population by what the ODE
 * advances in
 *
 * ```text
 * dt = beta * m / 2
 * ```
 *
 * # Where it holds
 *
 * That last step is the weak-selection limit, and it is an approximation: for
 * large `beta * m * dPi` the tanh saturates at 1 while the linear term keeps
 * growing, so a strongly selecting population moves *slower* than this mapping
 * predicts and the overlay would settle first. The curated default keeps
 * `beta * m * dPi` at or below 0.5, where the two agree to within 2%, which is
 * far inside the noise of a finite population. Selection strength is the
 * parameter to be careful with, not a detail.
 *
 * # One step, one generation
 *
 * The trajectory is integrated with exactly one RK4 step per generation, so
 * the two histories share an index as well as a scale and the chart can draw
 * them against one x axis with no resampling.
 *
 * # Where this belongs
 *
 * Here, for now, because it is a statement about how to *draw* two models
 * together. When Issue 5a makes the update rule selectable, the mapping stops
 * being a property of the parameters and becomes a property of the rule, and
 * it should move into `sim-core` next to the rule it belongs to.
 */
export function dtPerGeneration(params: TimeMappingParams): number {
  return (params.selectionStrength * params.matchesPerAgent) / 2;
}
