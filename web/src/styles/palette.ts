/**
 * Strategy colours: the one part of the palette that three consumers need in
 * three different forms - CSS (legends, swatches), canvas 2D (the charts), and
 * GLSL (the 3b grid shader). They live here rather than in `tokens.css` so
 * there is a single source: the shader and the legend cannot drift.
 *
 * Slot order matches the strategy ids `sim-core` uses, so slot `i` colours
 * strategy `i` with no lookup table in between: for Hawk-Dove that is hawk 0,
 * dove 1. Colour follows the strategy, never its current share or rank - a
 * hawk is ember whether it is winning or nearly extinct.
 *
 * Hues are assigned in this fixed order and never cycled. Four slots is the
 * cap, which is also the largest game the sandbox (Issue 7) offers.
 *
 * ## How these values were chosen
 *
 * Not by eye. Each slot sits inside the OKLCH lightness band for a dark
 * surface (L 0.48-0.67) with chroma above 0.10, and the set was checked for
 * colour-vision separation against `--surface-base` (#14120f):
 *
 * - Slots 1-3 clear the all-pairs test (worst pair dE 11.6 under protanopia,
 *   20.0 under normal vision). All-pairs is the test that matters for the
 *   spatial grid, where any two strategies can end up in touching cells.
 * - Adding slot 4 clears the adjacent-pair test but not all-pairs: gold beside
 *   ember is the weak pair under deuteranopia. That only arises in a
 *   four-strategy sandbox game on the grid, and the mitigation there is the
 *   legend plus direct labels, which those views carry anyway.
 *
 * Re-run the check before changing any value here.
 */

/** A strategy's colour in every form the app consumes it in. */
export interface SeriesColor {
  /** `#rrggbb`, for CSS and for canvas `strokeStyle`/`fillStyle`. */
  readonly hex: string;
  /**
   * sRGB components in `[0, 1]`, ready to hand a shader as a `vec3`.
   *
   * These are sRGB values, not linear ones. The 3b renderer writes them
   * straight to a non-sRGB framebuffer with no blending, so the byte that
   * reaches the screen is the byte in `hex`. Anything that starts blending or
   * lighting in linear space has to convert first, and must say so.
   */
  readonly unitRgb: readonly [number, number, number];
  /** 0-255 components, the form `rgba()` strings are built from. */
  readonly byteRgb: readonly [number, number, number];
  /**
   * A lighter step of the same hue, for the analytic ODE overlay (2b.7).
   *
   * The overlay is the same strategy, so it keeps the hue; it is a different
   * kind of claim, so it is drawn lighter and dashed. Hue alone would make two
   * lines of identical colour cross each other illegibly, and a neutral grey
   * would stop saying which strategy it predicts.
   */
  readonly analyticHex: string;
}

/** How many strategies the palette can colour. */
export const STRATEGY_SLOT_COUNT = 4;

/**
 * Parses `#rrggbb` into its components.
 *
 * A malformed value here is a typo in this file, not user input, so it throws
 * at module load rather than yielding a silently wrong colour.
 */
function parseHex(hex: string): readonly [number, number, number] {
  const match = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (match === null) {
    throw new RangeError(`palette: "${hex}" is not a #rrggbb colour`);
  }
  return [
    Number.parseInt(match[1], 16),
    Number.parseInt(match[2], 16),
    Number.parseInt(match[3], 16),
  ];
}

function series(hex: string, analyticHex: string): SeriesColor {
  const byteRgb = parseHex(hex);
  return {
    hex,
    byteRgb,
    unitRgb: [byteRgb[0] / 255, byteRgb[1] / 255, byteRgb[2] / 255],
    analyticHex,
  };
}

/**
 * The strategy palette, in fixed slot order: ember, sage, iris, gold.
 *
 * A tuple rather than an array so the length is part of the type and
 * `STRATEGY_SERIES[0]` is known to exist.
 */
export const STRATEGY_SERIES: readonly [
  SeriesColor,
  SeriesColor,
  SeriesColor,
  SeriesColor,
] = [
  series("#e4633a", "#f08f6b"), // ember - hawk in Hawk-Dove
  series("#2f9e88", "#56c3ac"), // sage - dove in Hawk-Dove
  series("#7b7ce0", "#a3a4ef"), // iris
  series("#b08a20", "#d3aa3c"), // gold
];

/**
 * The colour for strategy `index`.
 *
 * @throws RangeError if `index` is not a slot. Every game in the app declares
 * its strategy count up front and none exceeds {@link STRATEGY_SLOT_COUNT}, so
 * an out-of-range index is a bug in the caller. It fails loudly rather than
 * cycling the hues, which would give two strategies the same colour.
 */
export function strategySeries(index: number): SeriesColor {
  if (!Number.isInteger(index) || index < 0 || index >= STRATEGY_SLOT_COUNT) {
    throw new RangeError(
      `palette: no colour for strategy ${index}; the palette has ${STRATEGY_SLOT_COUNT} slots`,
    );
  }
  return STRATEGY_SERIES[index];
}

/**
 * Builds a `rgba()` string for a strategy, for glows and fills under a line.
 *
 * Returns a string, so a caller that needs one every frame should hoist it -
 * canvas styles are set from strings, and building them in a frame callback is
 * the easiest way to allocate in the hot path.
 */
export function seriesRgba(color: SeriesColor, alpha: number): string {
  const [r, g, b] = color.byteRgb;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * The palette flattened to `[r, g, b, r, g, b, ...]` in `[0, 1]`, sized for a
 * `uniform3fv` array of {@link STRATEGY_SLOT_COUNT} `vec3`s.
 *
 * Allocates, so the 3b renderer calls it once at setup and keeps the result;
 * the palette never changes at runtime.
 */
export function strategyPaletteRgb(): Float32Array {
  const flat = new Float32Array(STRATEGY_SLOT_COUNT * 3);
  STRATEGY_SERIES.forEach((color, slot) => {
    flat.set(color.unitRgb, slot * 3);
  });
  return flat;
}

/**
 * Stamps the palette onto an element as `--strategy-{n}` and
 * `--strategy-{n}-analytic` custom properties, numbered from 1 to match how
 * the tokens read in CSS.
 *
 * This is the bridge that lets `tokens.css` stay the only stylesheet with
 * colour in it while these values keep living in TypeScript. The app shell
 * calls it once on the document element at startup.
 */
export function applyStrategyPalette(root: HTMLElement): void {
  STRATEGY_SERIES.forEach((color, slot) => {
    const n = slot + 1;
    root.style.setProperty(`--strategy-${n}`, color.hex);
    root.style.setProperty(`--strategy-${n}-analytic`, color.analyticHex);
  });
}
