/**
 * Driving a simulation from the browser's animation frames: play, pause, step
 * once, reset, and how many generations to run per frame.
 *
 * The driver owns the run and the `requestAnimationFrame` loop, and nothing
 * else does. That is what makes the unmount contract in the module registry
 * keepable: disposing the driver cancels the loop and frees the WASM run in
 * one call.
 */
import { createSignal, type Accessor } from "solid-js";

/**
 * What the driver needs from a run. `WellMixedRun` satisfies it, and so will
 * the spatial run in 3b - the driver is written against this, not against
 * Hawk-Dove.
 */
export interface DrivenRun {
  readonly generation: number;
  advance(generations: number): void;
  dispose(): void;
}

export type Playback = "playing" | "paused";

export interface RunDriverOptions<T extends DrivenRun> {
  /**
   * Builds a run. Called once now and again on every reset, which is why
   * reset is exact: it rebuilds from the seed instead of trying to walk a
   * stochastic run backwards, which is not something a stochastic run can do.
   */
  create(): T;
  /**
   * Draws the current state. Called once per frame while playing, and once
   * after any change that happened while paused.
   *
   * Called with the run rather than closing over it, so that switching runs on
   * reset cannot leave the renderer drawing the old one.
   */
  draw(run: T): void;
  /** Generations per frame. Defaults to 1. */
  stepsPerFrame?: number;
}

/**
 * # Allocation
 *
 * Nothing in the frame callback allocates: the callback is one arrow created
 * with the driver, the run and the renderer are read from fields, and the
 * signal updates carry numbers. The one allocation per generation happens
 * below this layer, in the array wasm-bindgen builds to carry a row of shares
 * across the boundary; {@link WellMixedRun.step} documents why it is there.
 *
 * # Failure
 *
 * A run that throws pauses the driver and lands in {@link error} rather than
 * throwing from an animation frame, where nothing can catch it and the loop
 * would keep rescheduling itself sixty times a second.
 */
export class RunDriver<T extends DrivenRun> {
  readonly #options: RunDriverOptions<T>;
  readonly #run: Accessor<T>;
  readonly #setRun: (run: T) => void;
  readonly #playback: Accessor<Playback>;
  readonly #setPlayback: (state: Playback) => void;
  readonly #generation: Accessor<number>;
  readonly #setGeneration: (generation: number) => void;
  readonly #stepsPerFrame: Accessor<number>;
  readonly #setStepsPerFrame: (steps: number) => void;
  readonly #error: Accessor<unknown>;
  readonly #setError: (error: unknown) => void;

  #frameHandle: number | null = null;
  #disposed = false;

  constructor(options: RunDriverOptions<T>) {
    this.#options = options;

    // Signals hold objects here, so the setters are wrapped: Solid reads a
    // bare function argument as an updater, and a run is not one.
    const [run, setRun] = createSignal<T>(options.create());
    this.#run = run;
    this.#setRun = (next) => setRun(() => next);

    const [playback, setPlayback] = createSignal<Playback>("paused");
    this.#playback = playback;
    this.#setPlayback = setPlayback;

    const [generation, setGeneration] = createSignal(run().generation);
    this.#generation = generation;
    this.#setGeneration = setGeneration;

    const [steps, setSteps] = createSignal(
      normaliseSteps(options.stepsPerFrame ?? 1),
    );
    this.#stepsPerFrame = steps;
    this.#setStepsPerFrame = setSteps;

    const [error, setError] = createSignal<unknown>(null);
    this.#error = error;
    this.#setError = (next) => setError(() => next);
  }

  /** The current run. A new one after every {@link reset}. */
  get run(): Accessor<T> {
    return this.#run;
  }

  /** Whether the loop is running. */
  get playback(): Accessor<Playback> {
    return this.#playback;
  }

  /** Generations run, updated once per frame rather than once per step. */
  get generation(): Accessor<number> {
    return this.#generation;
  }

  /** Generations advanced per frame. */
  get stepsPerFrame(): Accessor<number> {
    return this.#stepsPerFrame;
  }

  /** The failure that stopped the run, or `null`. */
  get error(): Accessor<unknown> {
    return this.#error;
  }

  /** Starts the loop. Does nothing if it is already playing or has failed. */
  play(): void {
    if (this.#disposed || this.#playback() === "playing") {
      return;
    }
    this.#setError(null);
    this.#setPlayback("playing");
    this.#schedule();
  }

  /** Stops the loop, leaving the run exactly where it is. */
  pause(): void {
    if (this.#playback() === "paused") {
      return;
    }
    this.#setPlayback("paused");
    this.#cancel();
    // The pending frame is cancelled, so the last state gets its own draw.
    this.requestDraw();
  }

  toggle(): void {
    if (this.#playback() === "playing") {
      this.pause();
    } else {
      this.play();
    }
  }

  /**
   * Advances exactly one generation and pauses, for reading a stochastic run
   * one step at a time.
   */
  stepOnce(): void {
    if (this.#disposed) {
      return;
    }
    this.pause();
    const run = this.#run();
    if (!this.#advance(run, 1)) {
      return;
    }
    this.#setGeneration(run.generation);
    this.requestDraw();
  }

  /**
   * Throws the run away and builds a fresh one from the same parameters.
   *
   * Reset is a rebuild, not an undo: the same seed replays the same
   * generations, and nothing of the old run's state can survive into the new
   * one because the old one is freed here.
   */
  reset(): void {
    if (this.#disposed) {
      return;
    }
    const playing = this.#playback() === "playing";
    this.pause();
    // Built before the old one is freed: `create` takes user-supplied
    // parameters, and a rejected set must leave the driver holding the run it
    // already had rather than a freed one.
    let fresh: T;
    try {
      fresh = this.#options.create();
    } catch (error) {
      this.#setError(error);
      return;
    }
    this.#run().dispose();
    this.#setRun(fresh);
    this.#setGeneration(fresh.generation);
    this.#setError(null);
    if (playing) {
      this.play();
    } else {
      this.requestDraw();
    }
  }

  /** Sets generations per frame, clamped to a whole number of at least 1. */
  setStepsPerFrame(steps: number): void {
    this.#setStepsPerFrame(normaliseSteps(steps));
  }

  /**
   * Asks for one draw on the next frame.
   *
   * For changes that alter what is on screen without advancing the run - a
   * resize, a palette change, a paused reset. Repeated calls before the frame
   * arrives collapse into one.
   */
  requestDraw(): void {
    if (this.#disposed) {
      return;
    }
    this.#schedule();
  }

  /**
   * Cancels the loop and frees the run.
   *
   * Idempotent, and the only teardown a module has to remember: after this the
   * driver holds no frame callback and no WASM memory.
   */
  dispose(): void {
    if (this.#disposed) {
      return;
    }
    this.#disposed = true;
    this.#setPlayback("paused");
    this.#cancel();
    this.#run().dispose();
  }

  #schedule(): void {
    if (this.#frameHandle === null) {
      this.#frameHandle = requestAnimationFrame(this.#frame);
    }
  }

  #cancel(): void {
    if (this.#frameHandle !== null) {
      cancelAnimationFrame(this.#frameHandle);
      this.#frameHandle = null;
    }
  }

  /**
   * The frame callback. One arrow, created with the driver and reused for
   * every frame, so the hot path allocates nothing of its own.
   */
  readonly #frame = (): void => {
    this.#frameHandle = null;
    if (this.#disposed) {
      return;
    }
    const run = this.#run();
    if (this.#playback() === "playing") {
      if (!this.#advance(run, this.#stepsPerFrame())) {
        return;
      }
      this.#setGeneration(run.generation);
    }
    this.#options.draw(run);
    if (this.#playback() === "playing") {
      this.#schedule();
    }
  };

  /** Runs generations, pausing on failure. Returns whether it succeeded. */
  #advance(run: T, generations: number): boolean {
    try {
      run.advance(generations);
      return true;
    } catch (error) {
      this.#setPlayback("paused");
      this.#cancel();
      this.#setError(error);
      return false;
    }
  }
}

function normaliseSteps(steps: number): number {
  return Number.isFinite(steps) ? Math.max(1, Math.floor(steps)) : 1;
}
