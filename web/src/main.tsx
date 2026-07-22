/**
 * Entry point: load the simulation core, then render the shell.
 *
 * The core is awaited before the first render rather than loaded behind a
 * spinner. It is a 76kB WASM module served from the same origin, and every
 * module needs it immediately, so a loading state would flash more often than
 * it would inform.
 */
import { render } from "solid-js/web";
import "./styles/tokens.css";
import "./styles/base.css";
import { App } from "./app/App";
import { initCore } from "./core";
import { applyStrategyPalette } from "./styles/palette";

async function start(root: HTMLElement): Promise<void> {
  applyStrategyPalette(document.documentElement);
  await initCore();
  render(() => <App />, root);
}

/**
 * Reports a core that would not load. Nothing in the app works without it, so
 * this replaces the page rather than annotating it.
 */
function reportFailure(root: HTMLElement, error: unknown): void {
  const message = error instanceof Error ? error.message : String(error);
  root.textContent = `Could not load the simulation core: ${message}`;
  root.style.padding = "var(--space-8)";
  root.style.color = "var(--ink-secondary)";
  root.style.fontFamily = "var(--font-mono)";
  root.style.fontSize = "var(--text-sm)";
}

const root = document.getElementById("root");
if (root === null) {
  throw new Error('index.html must provide <div id="root">');
}

start(root).catch((error: unknown) => {
  reportFailure(root, error);
});
