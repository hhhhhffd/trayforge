import type { KeyboardEvent, PointerEvent } from "react";

const KEY_STEP = 16;

interface Props {
  /** "vertical" is a vertical bar that resizes widths (col-resize); "horizontal" resizes heights. */
  orientation: "vertical" | "horizontal";
  label: string;
  value: number;
  min: number;
  /** A function for limits that depend on the live layout (e.g. half the window). */
  max: number | (() => number);
  defaultValue: number;
  /** The pane grows when the bar moves left/up (the OPTIONS panel sits to the right of its bar). */
  invert?: boolean;
  className?: string;
  onChange: (value: number) => void;
}

export function Splitter({ orientation, label, value, min, max, defaultValue, invert = false, className, onChange }: Props) {
  const vertical = orientation === "vertical";
  const limit = () => Math.max(min, typeof max === "function" ? max() : max);
  const set = (v: number) => onChange(Math.round(Math.min(limit(), Math.max(min, v))));
  // +1 when moving right/down grows the pane.
  const sign = invert ? -1 : 1;

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    const bar = e.currentTarget;
    bar.setPointerCapture(e.pointerId);
    const startPos = vertical ? e.clientX : e.clientY;
    const startValue = value;
    const move = (ev: globalThis.PointerEvent) => {
      set(startValue + sign * ((vertical ? ev.clientX : ev.clientY) - startPos));
    };
    const end = () => {
      bar.removeEventListener("pointermove", move);
      bar.removeEventListener("pointerup", end);
      bar.removeEventListener("pointercancel", end);
      bar.classList.remove("dragging");
    };
    bar.classList.add("dragging");
    bar.addEventListener("pointermove", move);
    bar.addEventListener("pointerup", end);
    bar.addEventListener("pointercancel", end);
  };

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const grow = vertical ? "ArrowRight" : "ArrowDown";
    const shrink = vertical ? "ArrowLeft" : "ArrowUp";
    if (e.key === grow) set(value + sign * KEY_STEP);
    else if (e.key === shrink) set(value - sign * KEY_STEP);
    else if (e.key === "Home") set(min);
    else if (e.key === "End") set(limit());
    else if (e.key === "Enter") set(defaultValue);
    else return;
    e.preventDefault();
  };

  return (
    <div
      className={`splitter ${vertical ? "v" : "h"}${className ? ` ${className}` : ""}`}
      role="separator"
      aria-orientation={vertical ? "vertical" : "horizontal"}
      aria-label={label}
      aria-valuenow={Math.round(value)}
      aria-valuemin={min}
      aria-valuemax={limit()}
      tabIndex={0}
      title="Drag to resize, double-click to reset"
      onPointerDown={onPointerDown}
      onDoubleClick={() => set(defaultValue)}
      onKeyDown={onKeyDown}
    />
  );
}
