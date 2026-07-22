/**
 * The hub: every module as a card, in the order the catalogue lists them,
 * which is the recommended path. One card is signposted as the entry point;
 * modules that do not exist yet are listed but not openable, so the path is
 * visible before it is finished.
 */
import { For, Show, type JSX } from "solid-js";
import { MODULES } from "../modules";
import type { ModuleEntry, ReadyModule } from "../modules/registry";
import styles from "./Hub.module.css";

interface HubProps {
  onOpen(entry: ReadyModule): void;
}

export function Hub(props: HubProps): JSX.Element {
  return (
    <div class={styles.hub}>
      <div class={styles.intro}>
        <h2 class={styles.title}>Evolutionary game theory, run live</h2>
        <p class={styles.lede}>
          Populations play, strategies spread, and equilibria appear on their
          own. Open a module and change the rules.
        </p>
      </div>

      <ul class={styles.grid}>
        <For each={MODULES}>
          {(entry) => (
            <li>
              <ModuleCard entry={entry} onOpen={props.onOpen} />
            </li>
          )}
        </For>
      </ul>
    </div>
  );
}

interface ModuleCardProps {
  readonly entry: ModuleEntry;
  onOpen(entry: ReadyModule): void;
}

function ModuleCard(props: ModuleCardProps): JSX.Element {
  const body = (): JSX.Element => (
    <>
      <p class={styles.concept}>{props.entry.concept}</p>
      <h3 class={styles.cardTitle}>{props.entry.title}</h3>
      <p class={styles.summary}>{props.entry.summary}</p>
    </>
  );

  return (
    <Show
      when={props.entry.state === "ready" ? props.entry : null}
      fallback={
        <div class={`${styles.card} ${styles.planned}`}>
          {body()}
          <p class={styles.tag}>Planned</p>
        </div>
      }
    >
      {(ready) => (
        <button
          type="button"
          class={styles.card}
          onClick={() => props.onOpen(ready())}
        >
          {body()}
          <Show when={ready().startHere === true}>
            <p class={`${styles.tag} ${styles.startHere}`}>Start here</p>
          </Show>
        </button>
      )}
    </Show>
  );
}
