/**
 * Loading the WASM core.
 *
 * Together with the wrappers beside it, this file is the only part of the app
 * that imports `sim-core/pkg`.
 */
import init, { core_version } from "../../../sim-core/pkg/sim_core";
import { CoreError } from "./errors";

let loaded = false;

/**
 * Loads and instantiates the WASM core. Safe to call more than once; the
 * second call is a no-op rather than a second instantiation.
 *
 * Everything else in `core/` requires this to have resolved first.
 */
export async function initCore(): Promise<void> {
  if (loaded) {
    return;
  }
  await init();
  loaded = true;
}

/**
 * Guards an entry point that needs the core.
 *
 * @throws CoreError naming the caller. Without this, using the core early
 * fails inside wasm-bindgen with a message about an undefined import, which
 * points nowhere near the cause.
 */
export function assertCoreLoaded(what: string): void {
  if (!loaded) {
    throw new CoreError(`${what} needs the core: await initCore() first`);
  }
}

/**
 * The core's version string, for the shell header - it is how you confirm the
 * browser is running the WASM you just built rather than a cached one.
 */
export function coreVersion(): string {
  assertCoreLoaded("coreVersion");
  return core_version();
}
