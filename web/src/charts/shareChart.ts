/**
 * The share chart: strategy shares over the whole run, drawn on a 2D canvas.
 *
 * Imperative on purpose. It is handed a canvas and driven from the run
 * driver's frame callback, so the reactive layer never sees a redraw and the
 * hot path is a plain function call.
 *
 * # Reading the history
 *
 * The buffer is walked by stride: generation `g`'s share of strategy `s` is
 * `data[g * strategyCount + s]`. Nothing is mapped, sliced or copied per
 * frame - those all allocate, and at sixty frames a second that is the
 * difference between a chart and a garbage collector.
 *
 * # The whole run, not a window
 *
 * The x axis always spans generation 0 to now, so what the chart shows is the
 * shape of the entire run: the early swing, the settling, and the noise it
 * settles into. A scrolling window would hide exactly the part that makes the
 * point.
 *
 * Once there are more generations than pixel columns, each column summarises
 * its generations as a min/max band with the mean drawn through it. That keeps
 * the noise visible - decimating by sampling every nth generation would thin
 * the band as the run got longer and quietly make a noisy run look calm.
 *
 * # The y axis
 *
 * Pinned to [0, 1] always. These are shares of a population; an axis that
 * rescaled to the data would turn a settled run into a dramatic-looking one.
 */
import type { ShareHistory } from "../core";
import { seriesRgba, strategySeries } from "../styles/palette";

/** Plot area insets, in CSS pixels: room for the axis labels. */
const PAD_LEFT = 34;
const PAD_RIGHT = 10;
const PAD_TOP = 10;
const PAD_BOTTOM = 20;

/** Horizontal grid lines, and which of them carry a label. */
const GRID_LINES = [0, 0.25, 0.5, 0.75, 1] as const;
const LABELLED = [0, 0.5, 1] as const;

const SERIES_WIDTH = 2;
const GLOW_WIDTH = 7;
const GLOW_ALPHA = 0.14;
const BAND_ALPHA = 0.22;

interface Chrome {
  readonly grid: string;
  readonly axis: string;
  readonly label: string;
  readonly font: string;
}

export interface ShareChartOptions {
  /**
   * Which strategies to draw, by id. Omitted means all of them.
   *
   * Worth naming rather than always drawing everything: in a two-strategy game
   * the second series is the exact complement of the first, so drawing both
   * doubles the ink, and the two noise bands overlap into a grey smear around
   * the equilibrium. The complement is still on screen - as a number in the
   * readouts, which is where it is legible.
   */
  readonly series?: readonly number[];
}

export class ShareChart {
  readonly #canvas: HTMLCanvasElement;
  readonly #context: CanvasRenderingContext2D;
  readonly #observer: ResizeObserver;
  readonly #series: readonly number[] | null;

  #width = 0;
  #height = 0;
  #chrome: Chrome;
  #last: ShareHistory | null = null;

  /**
   * Per-column summaries, sized to the plot width and reused across frames and
   * strategies. Reallocated on resize only, which is what keeps the draw
   * allocation-free.
   */
  #columnMin = new Float32Array(0);
  #columnMax = new Float32Array(0);
  #columnMean = new Float32Array(0);

  /** Style strings per strategy, built once: building them per frame allocates. */
  readonly #stroke: string[] = [];
  readonly #glow: string[] = [];
  readonly #band: string[] = [];

  constructor(canvas: HTMLCanvasElement, options: ShareChartOptions = {}) {
    const context = canvas.getContext("2d");
    if (context === null) {
      throw new Error("share chart: this browser has no 2D canvas context");
    }
    this.#canvas = canvas;
    this.#context = context;
    this.#series = options.series ?? null;
    this.#chrome = readChrome(canvas);

    this.#observer = new ResizeObserver(this.#onResize);
    this.#observer.observe(canvas);
    this.#resize();
  }

  /**
   * Draws a history. Safe to call every frame; safe to call with the same
   * history twice.
   */
  draw(history: ShareHistory): void {
    this.#last = history;
    this.#render();
  }

  /** Stops observing the canvas. The canvas itself belongs to the caller. */
  dispose(): void {
    this.#observer.disconnect();
    this.#last = null;
  }

  readonly #onResize = (): void => {
    this.#resize();
    this.#render();
  };

  #resize(): void {
    const rect = this.#canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) {
      return;
    }
    // Backing store in device pixels, drawing in CSS pixels: crisp hairlines
    // on a retina display without any coordinate arithmetic below.
    const ratio = window.devicePixelRatio;
    this.#width = rect.width;
    this.#height = rect.height;
    this.#canvas.width = Math.round(rect.width * ratio);
    this.#canvas.height = Math.round(rect.height * ratio);
    this.#context.setTransform(ratio, 0, 0, ratio, 0, 0);
    this.#chrome = readChrome(this.#canvas);

    const columns = Math.max(0, Math.floor(rect.width - PAD_LEFT - PAD_RIGHT));
    if (columns !== this.#columnMin.length) {
      this.#columnMin = new Float32Array(columns);
      this.#columnMax = new Float32Array(columns);
      this.#columnMean = new Float32Array(columns);
    }
  }

  #render(): void {
    const history = this.#last;
    const ctx = this.#context;
    if (this.#width === 0 || this.#height === 0) {
      return;
    }
    ctx.clearRect(0, 0, this.#width, this.#height);

    const plotWidth = this.#width - PAD_LEFT - PAD_RIGHT;
    const plotHeight = this.#height - PAD_TOP - PAD_BOTTOM;
    if (plotWidth <= 0 || plotHeight <= 0) {
      return;
    }
    this.#drawGrid(plotWidth, plotHeight);
    if (history === null || history.recordedGenerations === 0) {
      return;
    }
    this.#ensureStyles(history.strategyCount);

    const generations = history.recordedGenerations;
    const count = this.#series?.length ?? history.strategyCount;
    for (let index = 0; index < count; index += 1) {
      const strategy = this.#series?.[index] ?? index;
      if (strategy >= history.strategyCount) {
        continue;
      }
      if (generations > plotWidth) {
        this.#drawDecimated(history, strategy, plotWidth, plotHeight);
      } else {
        this.#drawExact(history, strategy, plotWidth, plotHeight);
      }
    }
    this.#drawGenerationLabel(generations - 1, plotWidth, plotHeight);
  }

  #drawGrid(plotWidth: number, plotHeight: number): void {
    const ctx = this.#context;
    ctx.lineWidth = 1;
    ctx.strokeStyle = this.#chrome.grid;
    ctx.beginPath();
    for (const value of GRID_LINES) {
      // Half-pixel offset so a 1px line lands on one row of pixels.
      const y = Math.round(PAD_TOP + (1 - value) * plotHeight) + 0.5;
      ctx.moveTo(PAD_LEFT, y);
      ctx.lineTo(PAD_LEFT + plotWidth, y);
    }
    ctx.stroke();

    ctx.strokeStyle = this.#chrome.axis;
    ctx.beginPath();
    const axisX = Math.round(PAD_LEFT) + 0.5;
    ctx.moveTo(axisX, PAD_TOP);
    ctx.lineTo(axisX, PAD_TOP + plotHeight);
    ctx.stroke();

    ctx.fillStyle = this.#chrome.label;
    ctx.font = this.#chrome.font;
    ctx.textAlign = "right";
    ctx.textBaseline = "middle";
    for (const value of LABELLED) {
      const y = PAD_TOP + (1 - value) * plotHeight;
      ctx.fillText(value.toFixed(1), PAD_LEFT - 8, y);
    }
  }

  /** One point per generation, while they still fit in the plot's columns. */
  #drawExact(
    history: ShareHistory,
    strategy: number,
    plotWidth: number,
    plotHeight: number,
  ): void {
    const ctx = this.#context;
    const data = history.data;
    const stride = history.strategyCount;
    const generations = history.recordedGenerations;
    const span = Math.max(1, generations - 1);

    ctx.beginPath();
    for (let g = 0; g < generations; g += 1) {
      const x = PAD_LEFT + (g / span) * plotWidth;
      const y = PAD_TOP + (1 - data[g * stride + strategy]) * plotHeight;
      if (g === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    this.#strokeSeries(strategy);
  }

  /**
   * One column per pixel: the min/max band of the generations that fall in it,
   * with their mean drawn through it.
   */
  #drawDecimated(
    history: ShareHistory,
    strategy: number,
    plotWidth: number,
    plotHeight: number,
  ): void {
    const ctx = this.#context;
    const data = history.data;
    const stride = history.strategyCount;
    const generations = history.recordedGenerations;
    const columns = Math.min(this.#columnMin.length, Math.floor(plotWidth));
    if (columns === 0) {
      return;
    }

    for (let column = 0; column < columns; column += 1) {
      const from = Math.floor((column * generations) / columns);
      const to = Math.max(from + 1, Math.floor(((column + 1) * generations) / columns));
      let min = Number.POSITIVE_INFINITY;
      let max = Number.NEGATIVE_INFINITY;
      let sum = 0;
      for (let g = from; g < to; g += 1) {
        const value = data[g * stride + strategy];
        if (value < min) {
          min = value;
        }
        if (value > max) {
          max = value;
        }
        sum += value;
      }
      this.#columnMin[column] = min;
      this.#columnMax[column] = max;
      this.#columnMean[column] = sum / (to - from);
    }

    // The band: forward along the maxima, back along the minima.
    ctx.beginPath();
    for (let column = 0; column < columns; column += 1) {
      const x = PAD_LEFT + column;
      const y = PAD_TOP + (1 - this.#columnMax[column]) * plotHeight;
      if (column === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    for (let column = columns - 1; column >= 0; column -= 1) {
      const x = PAD_LEFT + column;
      ctx.lineTo(x, PAD_TOP + (1 - this.#columnMin[column]) * plotHeight);
    }
    ctx.closePath();
    ctx.fillStyle = this.#band[strategy];
    ctx.fill();

    ctx.beginPath();
    for (let column = 0; column < columns; column += 1) {
      const x = PAD_LEFT + column;
      const y = PAD_TOP + (1 - this.#columnMean[column]) * plotHeight;
      if (column === 0) {
        ctx.moveTo(x, y);
      } else {
        ctx.lineTo(x, y);
      }
    }
    this.#strokeSeries(strategy);
  }

  /** A wide, faint pass under a thin solid one: the glow, without a filter. */
  #strokeSeries(strategy: number): void {
    const ctx = this.#context;
    ctx.lineJoin = "round";
    ctx.lineCap = "round";
    ctx.lineWidth = GLOW_WIDTH;
    ctx.strokeStyle = this.#glow[strategy];
    ctx.stroke();
    ctx.lineWidth = SERIES_WIDTH;
    ctx.strokeStyle = this.#stroke[strategy];
    ctx.stroke();
  }

  #drawGenerationLabel(
    generation: number,
    plotWidth: number,
    plotHeight: number,
  ): void {
    const ctx = this.#context;
    ctx.fillStyle = this.#chrome.label;
    ctx.font = this.#chrome.font;
    ctx.textBaseline = "top";
    const y = PAD_TOP + plotHeight + 6;
    ctx.textAlign = "left";
    ctx.fillText("0", PAD_LEFT, y);
    ctx.textAlign = "right";
    ctx.fillText(`generation ${generation}`, PAD_LEFT + plotWidth, y);
  }

  /** Builds the per-strategy style strings the first time they are needed. */
  #ensureStyles(strategyCount: number): void {
    for (let strategy = this.#stroke.length; strategy < strategyCount; strategy += 1) {
      const color = strategySeries(strategy);
      this.#stroke.push(color.hex);
      this.#glow.push(seriesRgba(color, GLOW_ALPHA));
      this.#band.push(seriesRgba(color, BAND_ALPHA));
    }
  }
}

/**
 * Reads the chart's chrome colours and label font from the design tokens, so
 * the canvas and the DOM around it stay one palette.
 *
 * Called on construction and on resize, never per frame: `getComputedStyle`
 * forces layout, which is not something to do sixty times a second.
 */
function readChrome(element: HTMLElement): Chrome {
  const style = window.getComputedStyle(element);
  const token = (name: string, fallback: string): string => {
    const value = style.getPropertyValue(name).trim();
    return value === "" ? fallback : value;
  };
  return {
    grid: token("--chart-grid", "#241f19"),
    axis: token("--chart-axis", "#3a352b"),
    label: token("--chart-label", "#8a7f70"),
    font: `${token("--text-xs", "11px")} ${token("--font-mono", "monospace")}`,
  };
}
