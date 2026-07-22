/**
 * The Hawk-Dove module: an agent-based population playing the game live, with
 * the analytic replicator trajectory overlaid on the same chart, and the
 * parameters behind a disclosure.
 *
 * The readout strip becomes the proper live numbers in 2b.9.
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
import { ReplicatorTrajectory, WellMixedRun } from "../../core";
import { RunDriver } from "../../sim/driver";
import { dtPerGeneration } from "../../sim/timeMapping";
import { strategySeries } from "../../styles/palette";
import { Disclosure } from "../../ui/Disclosure";
import { Slider } from "../../ui/Slider";
import { DEFAULT_PARAMS, RANGES } from "./parameters";
import styles from "./HawkDoveModule.module.css";

/**
 * Strategy ids as `sim-core` numbers them. The names are presentation: the
 * core knows a payoff matrix, not a bird.
 */
const HAWK = 0;
const DOVE = 1;

/**
 * ODE time per generation. Constant, because it depends only on selection
 * strength and matches per agent, and neither is under a slider: they set
 * whether the run is in the regime the overlay is valid in, which is not a
 * knob to hand over without the explanation that goes with it.
 */
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
  const [params, setParams] = createSignal(DEFAULT_PARAMS);

  /**
   * The analytic overlay. Built once per parameter change, from the same V, C
   * and starting share as the run, and integrated ahead of it rather than
   * recomputed per frame: it is deterministic, so neither a step nor a reset
   * changes it.
   */
  let trajectory = ReplicatorTrajectory.create(DEFAULT_PARAMS);
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
    create: () => WellMixedRun.create(params()),
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

  /**
   * Applies one changed parameter: the trajectory is rebuilt for the new game
   * and the run is rebuilt from the seed.
   *
   * Both models restart, because neither can be steered mid-flight - a run's
   * agents are already playing the old payoff matrix, and pretending otherwise
   * would show a curve that no single game produced. The new trajectory is
   * built before the old one is freed, so a rejected configuration leaves the
   * chart showing the last valid one.
   */
  const change = <Key extends keyof typeof DEFAULT_PARAMS>(
    key: Key,
    value: (typeof DEFAULT_PARAMS)[Key],
  ): void => {
    const next = { ...params(), [key]: value };
    setParams(next);
    try {
      const fresh = ReplicatorTrajectory.create(next);
      trajectory.dispose();
      trajectory = fresh;
      horizonFailed = false;
      setOverlayFailure(null);
      extendTrajectory(0);
    } catch (error) {
      setOverlayFailure(error instanceof Error ? error.message : String(error));
    }
    driver.reset();
  };

  const shareOf = (strategy: number): number => {
    driver.generation();
    return driver.run().history.latest(strategy);
  };
  /** What the run is configured with, for the closed parameter panel. */
  const configuration = (): string => {
    const current = params();
    return `V ${current.v.toFixed(1)} · C ${current.c.toFixed(1)} · N ${current.population} · seed ${current.seed}`;
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

          <Disclosure summary="Parameters" detail={configuration()}>
            <Slider
              label="Resource value V"
              value={params().v}
              min={RANGES.v.min}
              max={RANGES.v.max}
              step={RANGES.v.step}
              format={(value) => value.toFixed(1)}
              onInput={(value) => change("v", value)}
            />
            <Slider
              label="Fight cost C"
              value={params().c}
              min={RANGES.c.min}
              max={RANGES.c.max}
              step={RANGES.c.step}
              format={(value) => value.toFixed(1)}
              onInput={(value) => change("c", value)}
            />
            <Slider
              label="Population N"
              value={params().population}
              min={RANGES.population.min}
              max={RANGES.population.max}
              step={RANGES.population.step}
              onInput={(value) => change("population", value)}
            />
            <Slider
              label="Starting hawks"
              value={params().initialHawkShare}
              min={RANGES.initialHawkShare.min}
              max={RANGES.initialHawkShare.max}
              step={RANGES.initialHawkShare.step}
              format={(value) => `${Math.round(value * 100)}%`}
              onInput={(value) => change("initialHawkShare", value)}
            />
            <Slider
              label="Seed"
              value={params().seed}
              min={RANGES.seed.min}
              max={RANGES.seed.max}
              step={RANGES.seed.step}
              hint="The same seed and parameters replay the same run."
              onInput={(value) => change("seed", value)}
            />
          </Disclosure>
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
