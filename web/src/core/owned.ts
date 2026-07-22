/**
 * Ownership of a WASM object.
 *
 * Objects that live in WASM linear memory are invisible to the JavaScript
 * garbage collector: dropping the last reference to one leaks it for the
 * lifetime of the page. Every such object therefore has exactly one owner,
 * and that owner is this type.
 *
 * It does two things a bare reference cannot: it makes disposal idempotent,
 * and it turns use-after-dispose into a named error instead of wasm-bindgen's
 * "null pointer passed to rust", which says nothing about what went wrong.
 */
import { DisposedError } from "./errors";

/** The shape wasm-bindgen gives every exported class. */
interface Freeable {
  free(): void;
}

export class Owned<T extends Freeable> {
  #value: T | null;
  readonly #what: string;

  /** Takes ownership of `value`. `what` names it in error messages. */
  constructor(value: T, what: string) {
    this.#value = value;
    this.#what = what;
  }

  /** True until {@link dispose} runs. */
  get alive(): boolean {
    return this.#value !== null;
  }

  /**
   * The owned object.
   *
   * @throws DisposedError if it has already been freed. Callers hold the
   * result only for the duration of the call - keeping it across an `await` or
   * in a closure re-creates exactly the dangling reference this type exists to
   * prevent.
   */
  get(): T {
    if (this.#value === null) {
      throw new DisposedError(this.#what);
    }
    return this.#value;
  }

  /** Frees the object. Calling it again does nothing. */
  dispose(): void {
    if (this.#value === null) {
      return;
    }
    // Cleared first: if `free` throws, the pointer is still gone, and a
    // second attempt to free it would be a double free.
    const value = this.#value;
    this.#value = null;
    value.free();
  }
}
