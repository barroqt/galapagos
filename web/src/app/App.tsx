/**
 * The app shell: a persistent header, and one region below it that is either
 * the hub or a mounted module.
 *
 * Navigation state is a single signal holding the open module, not an id, so
 * "a module is open" and "which one" cannot disagree, and only a module the
 * type system knows is mountable can ever be in it.
 */
import { createSignal, Show, type JSX } from "solid-js";
import { Hub } from "./Hub";
import { ModuleHost } from "./ModuleHost";
import { coreVersion } from "../core";
import type { ReadyModule } from "../modules/registry";
import styles from "./App.module.css";

export function App(): JSX.Element {
  const [active, setActive] = createSignal<ReadyModule | null>(null);

  return (
    <div class={styles.app}>
      <header class={styles.header}>
        <h1 class={styles.wordmark}>
          <Show
            when={active() !== null}
            fallback={<span class={styles.wordmarkText}>Galápagos</span>}
          >
            <button
              type="button"
              class={styles.wordmarkText}
              onClick={() => setActive(null)}
            >
              Galápagos
            </button>
          </Show>
        </h1>
        <Show when={active()}>
          {(entry) => (
            <p class={styles.crumb}>
              <span class={styles.crumbMark} aria-hidden="true">
                /
              </span>
              {entry().title}
            </p>
          )}
        </Show>
        <span class={styles.headerSpacer} />
        <p class={styles.version}>{coreVersion()}</p>
      </header>

      <main class={styles.main}>
        <Show when={active()} fallback={<Hub onOpen={setActive} />}>
          {(entry) => <ModuleHost entry={entry()} />}
        </Show>
      </main>
    </div>
  );
}
