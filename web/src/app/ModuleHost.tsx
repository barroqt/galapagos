/**
 * Mounts the open module into a host element and, just as importantly, takes
 * it out again.
 *
 * The mount lives in a `createEffect` rather than an `onMount` so that
 * switching straight from one module to another is handled by the same code
 * path as closing one: the effect re-runs, its cleanup unmounts the outgoing
 * module before the incoming one is mounted. With `onMount` the component
 * would be reused across that change and the first module would keep running
 * behind the second - a leak that only shows up as a mysteriously busy CPU.
 */
import { createEffect, onCleanup, type JSX } from "solid-js";
import type { ReadyModule } from "../modules/registry";
import styles from "./ModuleHost.module.css";

interface ModuleHostProps {
  readonly entry: ReadyModule;
}

export function ModuleHost(props: ModuleHostProps): JSX.Element {
  let host: HTMLDivElement | undefined;

  createEffect(() => {
    const entry = props.entry;
    if (host === undefined) {
      return;
    }
    const handle = entry.mount(host);
    onCleanup(() => {
      handle.unmount();
    });
  });

  return <div class={styles.host} ref={host} />;
}
