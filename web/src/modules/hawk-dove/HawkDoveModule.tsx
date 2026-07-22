/**
 * The Hawk-Dove module: an agent-based population playing the game live, with
 * the analytic replicator trajectory overlaid on the same chart.
 *
 * This is the frame. The three regions it lays out are filled by the tasks
 * that follow: the run driver and share chart in the stage, parameter controls
 * in the rail, and live numbers in the readout strip.
 */
import type { JSX } from "solid-js";
import styles from "./HawkDoveModule.module.css";

export function HawkDoveModule(): JSX.Element {
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
          <p class={styles.pending}>Run controls</p>
        </aside>
      </div>

      <footer class={styles.readouts}>
        <p class={styles.pending}>Live readouts</p>
      </footer>
    </section>
  );
}
