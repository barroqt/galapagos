/**
 * The Hawk-Dove module: an agent-based population playing the game live, with
 * the analytic replicator trajectory overlaid on the same chart.
 *
 * This is the frame plus the run driver. The share chart lands in the stage in
 * 2b.6, the full parameter panel replaces the transport controls in the rail
 * in 2b.8, and the readout strip becomes the proper live numbers in 2b.9.
 */
import { createMemo, onCleanup, onMount, Show, type JSX } from "solid-js";
import { WellMixedRun, type HawkDoveParams } from "../../core";
import { RunDriver } from "../../sim/driver";
import { strategySeries } from "../../styles/palette";
import styles from "./HawkDoveModule.module.css";

/**
 * Strategy ids as `sim-core` numbers them. The names are presentation: the
 * core knows a payoff matrix, not a bird.
 */
const HAWK = 0;
const DOVE = 1;

/**
 * The curated default: V < C, so there is an interior equilibrium at V/C =
 * 0.5, and a population large enough to read a trend in but small enough to
 * stay visibly noisy. 2b.8 puts these under the sliders.
 */
const DEFAULT_PARAMS: HawkDoveParams = {
  v: 2,
  c: 4,
  population: 500,
  initialHawkShare: 0.5,
  seed: 42,
};

/** Offered speeds, in generations per frame. */
const SPEEDS = [1, 4, 16, 64] as const;

export function HawkDoveModule(): JSX.Element {
  const driver = new RunDriver({
    create: () => WellMixedRun.create(DEFAULT_PARAMS),
    // The chart takes this over in 2b.6. Until then the readouts are driven by
    // the generation signal, so there is nothing to paint.
    draw: () => {},
  });

  onMount(() => {
    driver.play();
  });
  // The whole unmount contract for this module: one call that cancels the
  // frame loop and frees the WASM run.
  onCleanup(() => {
    driver.dispose();
  });

  const shareOf = (strategy: number): number => {
    driver.generation();
    return driver.run().history.latest(strategy);
  };
  const failure = createMemo(() => {
    const error = driver.error();
    if (error === null) {
      return null;
    }
    return error instanceof Error ? error.message : String(error);
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
          <p class={styles.pending}>Share chart</p>
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
