/**
 * Progressive disclosure: a section that is closed until it is wanted.
 *
 * Built on `<details>`, so keyboard support, the open/closed state and screen
 * reader semantics are the platform's rather than ours. It does not animate -
 * UI chrome here is snappy and static, and only simulations move.
 *
 * The summary carries a second line for the current values inside it, so that
 * closing the panel does not hide what the run is configured with.
 */
import type { JSX } from "solid-js";
import styles from "./Disclosure.module.css";

export interface DisclosureProps {
  readonly summary: string;
  /** A compact rendering of what is inside, shown when closed and when open. */
  readonly detail?: string;
  readonly open?: boolean;
  readonly children: JSX.Element;
}

export function Disclosure(props: DisclosureProps): JSX.Element {
  return (
    <details class={styles.disclosure} open={props.open}>
      <summary class={styles.summary}>
        <span class={styles.marker} aria-hidden="true" />
        <span class={styles.text}>
          <span class={styles.title}>{props.summary}</span>
          {props.detail !== undefined && (
            <span class={styles.detail}>{props.detail}</span>
          )}
        </span>
      </summary>
      <div class={styles.body}>{props.children}</div>
    </details>
  );
}
