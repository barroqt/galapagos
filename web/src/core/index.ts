/**
 * The WASM boundary: the only module in the app allowed to import
 * `sim-core/pkg`. Everything above it consumes the typed wrappers exported
 * here, so there is exactly one place to look for what crosses, what is a copy
 * and what has to be freed.
 *
 * This file starts small on purpose - loading the core and reporting its
 * version is all the shell (2b.3) needs. Task 2b.4 grows it into the owned,
 * disposable wrappers around `WellMixedSim` and `ReplicatorSim`.
 */
import init, { core_version } from "../../../sim-core/pkg/sim_core";

let loaded = false;

/**
 * Loads and instantiates the WASM core. Safe to call more than once; the
 * second call is a no-op rather than a second instantiation.
 *
 * Every other export here requires this to have resolved first.
 */
export async function initCore(): Promise<void> {
  if (loaded) {
    return;
  }
  await init();
  loaded = true;
}

/**
 * The core's version string, for the shell footer - it is how you confirm the
 * browser is running the WASM you just built rather than a cached one.
 *
 * @throws Error if the core has not been loaded yet. wasm-bindgen would throw
 * here anyway, with a message about an undefined import that says nothing
 * about the cause.
 */
export function coreVersion(): string {
  if (!loaded) {
    throw new Error("core: initCore() must resolve before coreVersion()");
  }
  return core_version();
}
