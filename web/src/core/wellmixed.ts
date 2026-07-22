/**
 * The agent-based well-mixed run: `sim-core`'s `WellMixedSim`, owned and typed.
 */
import { WellMixedSim } from "../../../sim-core/pkg/sim_core";
import { CoreError, describeError } from "./errors";
import { ShareHistory } from "./history";
import { assertCoreLoaded } from "./init";
import { Owned } from "./owned";
import type { HawkDoveParams } from "./params";

/**
 * One live population playing Hawk-Dove.
 *
 * # Lifetime
 *
 * Owns WASM memory. Call {@link dispose} when the run is discarded - on
 * unmount, and on every reset or parameter change, since those build a new
 * run rather than mutating this one back into shape.
 *
 * # History
 *
 * The run keeps its own {@link ShareHistory} rather than reading the core's
 * on demand. `sim-core` hands out its history as a **copy** of the whole
 * buffer, deliberately: a zero-copy view would dangle the moment the history
 * outgrew its capacity and WASM memory moved. Copying the whole run once per
 * frame is exactly the per-frame allocation the chart is not allowed to do, so
 * instead each step appends its one new row here, and the chart reads this
 * buffer directly. The two histories hold the same numbers in the same layout;
 * this one is simply on the side of the boundary that can be read for free.
 */
export class WellMixedRun {
  readonly #sim: Owned<WellMixedSim>;
  readonly #history: ShareHistory;
  readonly #population: number;

  private constructor(sim: WellMixedSim) {
    this.#sim = new Owned(sim, "well-mixed run");
    this.#population = sim.population();
    this.#history = new ShareHistory(sim.strategy_count());
    // Generation 0, before any step, so the run and the analytic trajectory
    // start from the same recorded point.
    this.#history.append(sim.current_shares());
  }

  /**
   * Configures a run.
   *
   * @throws CoreError if `sim-core` rejects the parameters; the message names
   * the offending value.
   */
  static create(params: HawkDoveParams): WellMixedRun {
    assertCoreLoaded("WellMixedRun.create");
    if (!Number.isSafeInteger(params.seed) || params.seed < 0) {
      throw new CoreError(
        `could not configure the run: seed must be a non-negative whole number, got ${params.seed}`,
      );
    }
    try {
      return new WellMixedRun(
        WellMixedSim.hawk_dove(
          params.v,
          params.c,
          params.population,
          params.initialHawkShare,
          BigInt(params.seed),
          params.selectionStrength,
          params.matchesPerAgent,
        ),
      );
    } catch (error) {
      throw new CoreError(describeError(error), { cause: error });
    }
  }

  /** Number of agents. */
  get population(): number {
    return this.#population;
  }

  /** Number of strategies, which is the stride of the history. */
  get strategyCount(): number {
    return this.#history.strategyCount;
  }

  /**
   * Generations run so far. Generation 0 is the starting state, so this is one
   * less than the number of recorded generations.
   */
  get generation(): number {
    return this.#history.recordedGenerations - 1;
  }

  /** The share history, in the layout {@link ShareHistory} documents. */
  get history(): ShareHistory {
    return this.#history;
  }

  /**
   * Runs one generation: every agent plays its matches, then the population
   * updates at once.
   *
   * The one allocation per call is `current_shares()`, a `strategyCount`-long
   * array that wasm-bindgen creates to carry the new row across. That is the
   * shape of the boundary, and it is bounded by the strategy count rather than
   * the length of the run. If it ever shows up in a profile, the fix is on the
   * Rust side (write the row into a caller-provided buffer), not a workaround
   * here.
   *
   * @throws CoreError if the core rejects the step, or DisposedError if the
   * run has been disposed.
   */
  step(): void {
    const sim = this.#sim.get();
    try {
      sim.step();
    } catch (error) {
      throw new CoreError(
        `could not run generation ${this.generation + 1}: ${describeError(error)}`,
        { cause: error },
      );
    }
    this.#history.append(sim.current_shares());
  }

  /**
   * Runs `generations` generations. Used by the driver to put more than one
   * generation in a frame; it stops at the first failure, leaving the history
   * consistent with what actually ran.
   */
  advance(generations: number): void {
    for (let i = 0; i < generations; i += 1) {
      this.step();
    }
  }

  /** Frees the WASM run. Idempotent; the history stays readable afterwards. */
  dispose(): void {
    this.#sim.dispose();
  }
}
