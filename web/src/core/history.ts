/**
 * The share history both kinds of run expose: strategy shares over time, flat
 * and generation-major, in the layout `sim-core` records them in.
 *
 * Generation `g`'s shares occupy `strategyCount` entries starting at
 * `g * strategyCount`. That is the layout of the agent-based run and of the
 * analytic trajectory alike, which is what lets the chart draw both by walking
 * one buffer with one stride and no reshaping.
 */

/**
 * A growable flat buffer of shares.
 *
 * # Reading it
 *
 * {@link data} is the live buffer, not a copy and not a `subarray`, so reading
 * it allocates nothing and is safe to do every frame. Two rules come with
 * that:
 *
 * - Only the first `recordedGenerations * strategyCount` entries are real.
 *   The rest is spare capacity holding stale numbers.
 * - The reference is invalidated by {@link append} and {@link adopt}, because
 *   growing reallocates. Re-read `data` each frame; never hold it across a
 *   step.
 *
 * This mirrors the discipline a zero-copy view into WASM memory would need,
 * which is deliberate: if the buffer ever becomes such a view, callers written
 * to these rules keep working.
 */
export class ShareHistory {
  readonly strategyCount: number;
  #data: Float64Array;
  #generations = 0;

  /**
   * @param strategyCount stride of the buffer, at least 1.
   * @param capacity generations to reserve up front. The default covers a
   * couple of minutes of a 60fps run before the first growth.
   */
  constructor(strategyCount: number, capacity = 4096) {
    if (!Number.isInteger(strategyCount) || strategyCount < 1) {
      throw new RangeError(`history: strategy count must be 1 or more, got ${strategyCount}`);
    }
    this.strategyCount = strategyCount;
    this.#data = new Float64Array(strategyCount * capacity);
  }

  /** How many generations hold real data. */
  get recordedGenerations(): number {
    return this.#generations;
  }

  /** The flat buffer. Read the invalidation rules on the class first. */
  get data(): Float64Array {
    return this.#data;
  }

  /** One strategy's share in one generation, or `NaN` if it is not recorded. */
  shareAt(generation: number, strategy: number): number {
    if (
      generation < 0 ||
      generation >= this.#generations ||
      strategy < 0 ||
      strategy >= this.strategyCount
    ) {
      return Number.NaN;
    }
    return this.#data[generation * this.strategyCount + strategy];
  }

  /** One strategy's share in the most recent generation, `NaN` if empty. */
  latest(strategy: number): number {
    return this.shareAt(this.#generations - 1, strategy);
  }

  /**
   * Appends one generation, growing the buffer when it is full.
   *
   * Allocation-free except on growth, which doubles the capacity, so a run of
   * `n` generations reallocates `log2(n / capacity)` times and never in a
   * steady state.
   */
  append(row: Float64Array): void {
    if (row.length !== this.strategyCount) {
      throw new RangeError(
        `history: expected ${this.strategyCount} shares, got ${row.length}`,
      );
    }
    const end = this.#generations * this.strategyCount;
    if (end + this.strategyCount > this.#data.length) {
      const grown = new Float64Array(this.#data.length * 2);
      grown.set(this.#data);
      this.#data = grown;
    }
    this.#data.set(row, end);
    this.#generations += 1;
  }

  /**
   * Takes a whole history computed in one go, replacing anything recorded.
   *
   * This is how the analytic trajectory arrives: `sim-core` integrates it in a
   * single call and hands back the finished buffer, so there is nothing to
   * append to and no reason to copy it again.
   */
  adopt(flat: Float64Array): void {
    if (flat.length % this.strategyCount !== 0) {
      throw new RangeError(
        `history: ${flat.length} values is not a whole number of ${this.strategyCount}-strategy generations`,
      );
    }
    this.#data = flat;
    this.#generations = flat.length / this.strategyCount;
  }
}
