/**
 * The analytic replicator trajectory: `sim-core`'s `ReplicatorSim`, owned and
 * typed. This is the deterministic curve the agent-based run is compared
 * against.
 */
import { ReplicatorSim } from "../../../sim-core/pkg/sim_core";
import { CoreError, describeError } from "./errors";
import { ShareHistory } from "./history";
import { assertCoreLoaded } from "./init";
import { Owned } from "./owned";
import type { TrajectoryParams } from "./params";

/**
 * The replicator ODE for one Hawk-Dove game.
 *
 * # Lifetime
 *
 * Owns WASM memory; {@link dispose} when the overlay is discarded. It shares
 * no state with a {@link WellMixedRun} - two runs on screen cannot perturb
 * each other - and takes no seed, because nothing here is stochastic.
 *
 * # History
 *
 * Unlike the agent-based run, this one is integrated in a single batch and
 * then read once. `share_history()` copies the whole trajectory out of WASM,
 * and {@link ShareHistory.adopt} takes that array as-is rather than copying it
 * again. That copy happens once per parameter change, which is also the only
 * time the overlay changes - never per frame.
 */
export class ReplicatorTrajectory {
  readonly #ode: Owned<ReplicatorSim>;
  readonly #history: ShareHistory;

  private constructor(ode: ReplicatorSim) {
    this.#ode = new Owned(ode, "analytic trajectory");
    this.#history = new ShareHistory(ode.strategy_count(), 1);
    this.#history.adopt(ode.share_history());
  }

  /**
   * Configures a trajectory. Pass the same `v`, `c` and initial share as the
   * run it will be drawn against; {@link TrajectoryParams} is derived from the
   * run's parameters so those cannot drift apart.
   *
   * @throws CoreError if `sim-core` rejects the parameters.
   */
  static create(params: TrajectoryParams): ReplicatorTrajectory {
    assertCoreLoaded("ReplicatorTrajectory.create");
    try {
      return new ReplicatorTrajectory(
        ReplicatorSim.hawk_dove(params.v, params.c, params.initialHawkShare),
      );
    } catch (error) {
      throw new CoreError(describeError(error), { cause: error });
    }
  }

  /** Number of strategies, which is the stride of the trajectory. */
  get strategyCount(): number {
    return this.#history.strategyCount;
  }

  /** Steps integrated so far. */
  get generation(): number {
    return this.#history.recordedGenerations - 1;
  }

  /** The trajectory, in the same layout as a run's share history. */
  get history(): ShareHistory {
    return this.#history;
  }

  /**
   * Integrates `steps` steps of `dt`, extending the trajectory, and re-reads
   * it into the history.
   *
   * Call this once when the parameters change, not once per frame: the whole
   * point of the ODE is that its curve is known in advance rather than
   * discovered a frame at a time.
   *
   * @throws CoreError if `dt` is not finite and positive or is too large for
   * this game, in which case the message names the step that has to shrink and
   * the trajectory is left at the last state that was on the simplex.
   */
  integrate(steps: number, dt: number): void {
    const ode = this.#ode.get();
    try {
      ode.run(steps, dt);
    } catch (error) {
      throw new CoreError(
        `could not integrate ${steps} steps of ${dt}: ${describeError(error)}`,
        { cause: error },
      );
    }
    this.#history.adopt(ode.share_history());
  }

  /** Frees the WASM trajectory. Idempotent; the history stays readable. */
  dispose(): void {
    this.#ode.dispose();
  }
}
