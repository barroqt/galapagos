/**
 * The error types the core boundary raises.
 *
 * `sim-core` already produces messages that name the offending value; these
 * types add the context of what the app was trying to do and keep the original
 * error as `cause`, so nothing is hidden on the way up.
 */

/** Anything that went wrong at the WASM boundary. */
export class CoreError extends Error {
  constructor(message: string, options?: ErrorOptions) {
    super(message, options);
    this.name = "CoreError";
  }
}

/**
 * A disposed object was used again.
 *
 * Its own type because it means something different from a rejected
 * parameter: not "the run you asked for is impossible" but "the run you are
 * holding is gone", which is always a lifetime bug in the caller.
 */
export class DisposedError extends CoreError {
  constructor(what: string) {
    super(`${what} has been disposed and cannot be used again`);
    this.name = "DisposedError";
  }
}

/** Renders an unknown thrown value as text, since anything can be thrown. */
export function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
