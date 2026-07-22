/**
 * The Hawk-Dove module: an agent-based population playing the game live, with
 * the analytic replicator trajectory overlaid on the same chart.
 *
 * This is the frame plus the run driver. The share chart lands in the stage in
 * 2b.6, the full parameter panel replaces the transport controls in the rail
 * in 2b.8, and the readout strip becomes the proper live numbers in 2b.9.
 */
import {
  createMemo,
  createSignal,
  onCleanup,
  onMount,
  Show,
  type JSX,
} from "solid-js";
import { ShareChart } from "../../charts/shareChart";
import {
  ReplicatorTrajectory,
  WellMixedRun,
  type HawkDoveParams,
} from "../../core";
import { RunDriver } from "../../sim/driver";
import { dtPerGeneration, type TimeMappingParams } from "../../sim/timeMapping";
import { strategySeries } from "../../styles/palette";
import styles from "./HawkDoveModule.module.css";

/**
 * Strategy ids as `sim-core` numbers them. The names are presentation: the
 * core knows a payoff matrix, not a bird.
 */
const HAWK = 0;
const DOVE = 1;

/**
 * The curated default. 2b.8 puts these under the sliders.
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
const DEFAULT_PARAMS: HawkDoveParams & TimeMappingParams = {
  v: 2,
  c: 4,
  population: 500,
  initialHawkShare: 0.9,
  seed: 42,
  selectionStrength: 0.05,
  matchesPerAgent: 10,
};

/** ODE time per generation, derived from the parameters above. */
const DT = dtPerGeneration(DEFAULT_PARAMS);

/** Generations of trajectory to have ready before the run needs them. */
const INITIAL_HORIZON = 512;

/** Offered speeds, in generations per frame. */
const SPEEDS = [1, 4, 16, 64] as const;

/**
 * The chart draws the hawk share alone. With two strategies the dove share is
 * exactly its complement, so a second line would add ink and no information;
 * the readouts carry the number.
 */
const CHARTED = [HAWK] as const;

export function HawkDoveModule(): JSX.Element {
  let canvas: HTMLCanvasElement | undefined;
  let chart: ShareChart | null = null;
  const [overlayFailure, setOverlayFailure] = createSignal<string | null>(null);

  /**
   * The analytic overlay. Built once, from the same V, C and starting share as
   * the run, and integrated ahead of it rather than recomputed per frame: it
   * is deterministic, so a reset does not change it and neither does a step.
   */
  const trajectory = ReplicatorTrajectory.create(DEFAULT_PARAMS);
  let horizonFailed = false;

  /**
   * Integrates further ahead when the run catches up with the trajectory,
   * doubling the horizon each time so a long run costs a handful of
   * extensions rather than one per frame.
   */
  const extendTrajectory = (generation: number): void => {
    if (horizonFailed || generation < trajectory.generation) {
      return;
    }
    const target = Math.max(
      INITIAL_HORIZON,
      generation + 1,
      trajectory.generation * 2,
    );
    try {
      trajectory.integrate(target - trajectory.generation, DT);
    } catch (error) {
      // Stop trying rather than failing once per frame from here on.
      horizonFailed = true;
      setOverlayFailure(error instanceof Error ? error.message : String(error));
    }
  };

  const driver = new RunDriver({
    create: () => WellMixedRun.create(DEFAULT_PARAMS),
    // One arrow, created with the driver: the chart is looked up from a field
    // rather than closed over, because it does not exist until the canvas is
    // in the document.
    draw: (run) => {
      extendTrajectory(run.generation);
      chart?.draw(run.history, trajectory.history);
    },
  });

  onMount(() => {
    if (canvas !== undefined) {
      chart = new ShareChart(canvas, { series: CHARTED });
    }
    extendTrajectory(0);
    driver.play();
  });
  // The whole unmount contract for this module: the chart stops observing its
  // canvas, and the driver and the trajectory each free their WASM object.
  onCleanup(() => {
    chart?.dispose();
    chart = null;
    driver.dispose();
    trajectory.dispose();
  });

  const shareOf = (strategy: number): number => {
    driver.generation();
    return driver.run().history.latest(strategy);
  };
  const failure = createMemo(() => {
    const error = driver.error();
    if (error !== null) {
      return error instanceof Error ? error.message : String(error);
    }
    return overlayFailure();
  });

  return (
    <section class={styles.module}>
      <header class={styles.head}>
        <h2 class={styles.title}>Hawk and Dove</h2>
        <p class={styles.subtitle}>
          Two strategies contest one resource. Hawks escalate and pay for it
          when they meet each other; doves never fight and never win outright.
          Neither takes over.
        </p>
      </header>

      <div class={styles.body}>
        <div class={styles.stage}>
          <canvas class={styles.chart} ref={canvas} />
        </div>

        <aside class={styles.rail}>
          <h3 class={styles.railTitle}>Run</h3>
          <div class={styles.transport}>
            <button
              type="button"
              class={`${styles.button} ${styles.primary}`}
              onClick={() => driver.toggle()}
            >
              {driver.playback() === "playing" ? "Pause" : "Play"}
            </button>
            <button
              type="button"
              class={styles.button}
              onClick={() => driver.stepOnce()}
            >
              Step
            </button>
            <button
              type="button"
              class={styles.button}
              onClick={() => driver.reset()}
            >
              Reset
            </button>
          </div>

          <div class={styles.field}>
            <p class={styles.fieldLabel}>Generations per frame</p>
            <div class={styles.segments}>
              {SPEEDS.map((speed) => (
                <button
                  type="button"
                  class={styles.segment}
                  aria-pressed={driver.stepsPerFrame() === speed}
                  onClick={() => driver.setStepsPerFrame(speed)}
                >
                  {speed}
                </button>
              ))}
            </div>
          </div>
        </aside>
      </div>

      <footer class={styles.readouts}>
        <Readout label="Generation" value={driver.generation().toString()} />
        <Readout
          label="Hawks"
          value={shareOf(HAWK).toFixed(3)}
          color={strategySeries(HAWK).hex}
        />
        <Readout
          label="Doves"
          value={shareOf(DOVE).toFixed(3)}
          color={strategySeries(DOVE).hex}
        />
        <Show when={failure()}>
          {(message) => <p class={styles.failure}>{message()}</p>}
        </Show>
      </footer>
    </section>
  );
}

interface ReadoutProps {
  readonly label: string;
  readonly value: string;
  readonly color?: string;
}

function Readout(props: ReadoutProps): JSX.Element {
  return (
    <div class={styles.readout}>
      <p class={styles.readoutLabel}>
        <Show when={props.color}>
          {(color) => (
            <span class={styles.swatch} style={{ background: color() }} />
          )}
        </Show>
        {props.label}
      </p>
      <p class={styles.readoutValue}>{props.value}</p>
    </div>
  );
}
