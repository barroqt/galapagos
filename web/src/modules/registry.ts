/**
 * The module registry: what the hub lists, and how a module is put on screen
 * and taken off it again.
 *
 * A module is a self-contained lab for one concept. The shell knows nothing
 * about what is inside one - it knows how to mount it into a host element and
 * how to unmount it, and the contract for what unmounting has to release.
 */
import { render } from "solid-js/web";
import { createComponent, type Component } from "solid-js";

/** Identifies a module. Adding one means adding a member here first. */
export type ModuleId =
  | "hawk-dove"
  | "spatial"
  | "rock-paper-scissors"
  | "stag-hunt"
  | "sandbox";

/**
 * A mounted module, from the shell's point of view.
 *
 * # The unmount contract
 *
 * `unmount` must leave nothing running and nothing allocated:
 *
 * - every `requestAnimationFrame` loop cancelled,
 * - every listener and observer (window, document, `ResizeObserver`) removed,
 * - every WASM object freed through its owner's `dispose()`, because the
 *   JavaScript garbage collector does not track WASM linear memory and will
 *   never collect them for you.
 *
 * A module built with {@link solidModule} gets all three by registering them
 * with `onCleanup` inside its own component, which is why that is the only
 * mount helper the app has.
 */
export interface ModuleHandle {
  unmount(): void;
}

/** Puts a module into `host` and hands back the way to take it out. */
export type ModuleMount = (host: HTMLElement) => ModuleHandle;

interface ModuleDescription {
  readonly id: ModuleId;
  /** Module name, as shown on its hub card and in its header. */
  readonly title: string;
  /** The game-theory idea it teaches, in a few words. */
  readonly concept: string;
  /** One sentence on the hub card. The card is the only prose a module gets. */
  readonly summary: string;
}

/** A module that can be opened. */
export interface ReadyModule extends ModuleDescription {
  readonly state: "ready";
  readonly mount: ModuleMount;
  /**
   * Marks the module the hub signposts for a newcomer. Exactly one module
   * carries it, which the catalogue in `modules/index.ts` asserts.
   */
  readonly startHere?: true;
}

/**
 * A module the hub lists but cannot open yet, so the recommended path is
 * visible from the first release rather than appearing a module at a time.
 *
 * It has no `mount`, so "open a module that does not exist" is not a state the
 * shell can get into - it is rejected by the type, not by a runtime check.
 */
export interface PlannedModule extends ModuleDescription {
  readonly state: "planned";
}

export type ModuleEntry = ReadyModule | PlannedModule;

/**
 * Wraps a Solid component as a mountable module.
 *
 * `render` gives the module its own reactive root, so disposing it runs every
 * `onCleanup` the module registered and nothing of the shell's. That
 * isolation is the point: a module's teardown cannot be half-done because the
 * shell forgot to call something.
 */
export function solidModule(component: Component): ModuleMount {
  return (host: HTMLElement): ModuleHandle => {
    const dispose = render(() => createComponent(component, {}), host);
    return { unmount: dispose };
  };
}
