/**
 * A labelled slider with its value shown beside it.
 *
 * The value is always on screen, not just while dragging: an expert reading a
 * run needs to know what it was configured with, and a slider position is not
 * a number.
 */
import type { JSX } from "solid-js";
import styles from "./Slider.module.css";

export interface SliderProps {
  readonly label: string;
  readonly value: number;
  readonly min: number;
  readonly max: number;
  readonly step: number;
  /** Renders the value for display. Defaults to the bare number. */
  readonly format?: (value: number) => string;
  /** One short line under the label, for what the parameter means. */
  readonly hint?: string;
  onInput(value: number): void;
}

export function Slider(props: SliderProps): JSX.Element {
  const display = (): string => {
    const format = props.format;
    return format === undefined ? String(props.value) : format(props.value);
  };
  // Drives the filled part of the track, so the slider reads as a level
  // rather than a knob on a rail.
  const fill = (): string => {
    const span = props.max - props.min;
    const ratio = span === 0 ? 0 : (props.value - props.min) / span;
    return `${(ratio * 100).toFixed(2)}%`;
  };

  return (
    <label class={styles.slider}>
      <span class={styles.row}>
        <span class={styles.label}>{props.label}</span>
        <span class={styles.value}>{display()}</span>
      </span>
      <input
        type="range"
        class={styles.input}
        style={{ "--fill": fill() }}
        min={props.min}
        max={props.max}
        step={props.step}
        value={props.value}
        onInput={(event) => props.onInput(event.currentTarget.valueAsNumber)}
      />
      {props.hint !== undefined && <span class={styles.hint}>{props.hint}</span>}
    </label>
  );
}
