/**
 * What the Hawk-Dove module opens on, and how far each control can travel.
 */
import type { HawkDoveParams } from "../../core";
import type { TimeMappingParams } from "../../sim/timeMapping";

/**
 * The curated default.
 *
 * - `V < C`, so there is an interior equilibrium at `V/C = 0.5` to converge to.
 * - The run starts at 90% hawks, away from that equilibrium, because a run
 *   that starts on the answer has nothing to show. Hawks dominant, fighting
 *   costly, and the share falls back to half: that is the whole idea in one
 *   curve.
 * - 500 agents: enough that the trend is legible, few enough that the noise a
 *   finite population makes is visible against the analytic curve.
 * - Selection strength 0.05 over 10 matches keeps `beta * m * dPi` at or below
 *   0.5, which is the weak-selection regime where the replicator equation is
 *   the run's deterministic limit. See `sim/timeMapping.ts`: this is the
 *   parameter that decides whether the overlay is a prediction or a decoration.
 */
export const DEFAULT_PARAMS: HawkDoveParams & TimeMappingParams = {
  v: 2,
  c: 4,
  population: 500,
  initialHawkShare: 0.9,
  seed: 42,
  selectionStrength: 0.05,
  matchesPerAgent: 10,
};

export interface Range {
  readonly min: number;
  readonly max: number;
  readonly step: number;
}

/**
 * Slider ranges, chosen so that no position of any control produces a
 * configuration `sim-core` rejects. What the core refuses, and why each range
 * cannot reach it:
 *
 * - **Non-finite payoffs.** A range input yields a finite number between its
 *   bounds; there is no NaN and no infinity to produce.
 * - **A population below two.** The minimum here is 50, well clear, and a
 *   smaller one would also stop being a population worth averaging over.
 * - **A hawk share outside [0, 1].** The range is exactly [0, 1]; both
 *   endpoints are legal and meaningful (all doves, all hawks), so they are
 *   reachable on purpose.
 * - **A seed that is not a whole number.** Step 1 from an integer minimum.
 *
 * Two upper bounds are about usefulness rather than validity. `C` is kept off
 * zero because the equilibrium share `V/C` divides by it, and `V` and `C` are
 * kept in the same span so that crossing `V = C` - where the interior
 * equilibrium disappears and hawk takes over - is a couple of slider steps
 * away rather than an unreachable curiosity.
 */
export const RANGES = {
  v: { min: 0.5, max: 6, step: 0.1 },
  c: { min: 0.5, max: 6, step: 0.1 },
  population: { min: 50, max: 2000, step: 10 },
  initialHawkShare: { min: 0, max: 1, step: 0.01 },
  seed: { min: 0, max: 999, step: 1 },
} as const satisfies Record<string, Range>;
