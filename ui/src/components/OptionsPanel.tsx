import { SlidersHorizontal } from "lucide-react";
import type { OptionKind, OptionValue, ToolOption } from "../types";

export function defaultValue(kind: OptionKind): OptionValue {
  switch (kind.type) {
    case "picker":
    case "segmented":
      return kind.choices[kind.default];
    default:
      return kind.default;
  }
}

/** Saved values over declared defaults; unknown or wrongly typed saved values are ignored. */
export function resolveOptions(
  options: ToolOption[],
  saved: Record<string, OptionValue> | undefined,
): Record<string, OptionValue> {
  const out: Record<string, OptionValue> = {};
  for (const o of options) {
    const def = defaultValue(o.kind);
    const v = saved?.[o.id];
    const valid =
      v !== undefined &&
      typeof v === typeof def &&
      ((o.kind.type !== "picker" && o.kind.type !== "segmented") ||
        o.kind.choices.includes(v as string));
    out[o.id] = valid ? (v as OptionValue) : def;
  }
  return out;
}

interface Props {
  options: ToolOption[];
  values: Record<string, OptionValue>;
  width: number;
  onChange: (id: string, value: OptionValue) => void;
}

export function OptionsPanel({ options, values, width, onChange }: Props) {
  return (
    <aside className="options scroll" aria-label="Options" style={{ width }}>
      <h3>
        <SlidersHorizontal size={15} aria-hidden="true" /> OPTIONS
      </h3>
      {options.map((o) => (
        <OptionControl key={o.id} option={o} value={values[o.id]} onChange={(v) => onChange(o.id, v)} />
      ))}
    </aside>
  );
}

function OptionControl({
  option: o,
  value,
  onChange,
}: {
  option: ToolOption;
  value: OptionValue;
  onChange: (v: OptionValue) => void;
}) {
  const id = `opt-${o.id}`;
  const k = o.kind;
  const help = o.help ? <small className="opt-help">{o.help}</small> : null;

  switch (k.type) {
    case "toggle":
      return (
        <div className="opt" title={o.help ?? undefined}>
          <label className="check">
            <input type="checkbox" checked={value as boolean} onChange={(e) => onChange(e.target.checked)} />
            <span>{o.label}</span>
          </label>
          {help}
        </div>
      );
    case "segmented":
      return (
        <div className="opt" title={o.help ?? undefined}>
          <span className="opt-label" id={id}>
            {o.label}
          </span>
          <div className="seg" role="radiogroup" aria-labelledby={id}>
            {k.choices.map((c) => (
              <button
                key={c}
                role="radio"
                aria-checked={value === c}
                className={value === c ? "on" : ""}
                onClick={() => onChange(c)}
              >
                {c}
              </button>
            ))}
          </div>
          {help}
        </div>
      );
    case "picker":
      return (
        <div className="opt row" title={o.help ?? undefined}>
          <label className="opt-label" htmlFor={id}>
            {o.label}
          </label>
          <select id={id} value={value as string} onChange={(e) => onChange(e.target.value)}>
            {k.choices.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
          {help}
        </div>
      );
    case "slider":
      return (
        <div className="opt" title={o.help ?? undefined}>
          <label className="opt-label" htmlFor={id}>
            {o.label} <span className="muted">{String(value)}</span>
          </label>
          <input
            id={id}
            type="range"
            min={k.min}
            max={k.max}
            step={k.step}
            value={value as number}
            onChange={(e) => onChange(Number(e.target.value))}
          />
          {help}
        </div>
      );
    case "textField":
      return (
        <div className="opt" title={o.help ?? undefined}>
          <label className="opt-label" htmlFor={id}>
            {o.label}
          </label>
          <input
            id={id}
            className="field"
            type="text"
            value={value as string}
            placeholder={k.placeholder}
            spellCheck={false}
            onChange={(e) => onChange(e.target.value)}
          />
          {help}
        </div>
      );
    case "stepper": {
      const n = value as number;
      const set = (next: number) => {
        if (Number.isFinite(next)) onChange(Math.min(k.max, Math.max(k.min, Math.trunc(next))));
      };
      return (
        <div className="opt row" title={o.help ?? undefined}>
          <label className="opt-label" htmlFor={id}>
            {o.label}
          </label>
          <div className="stepper">
            <button aria-label={`Decrease ${o.label}`} disabled={n <= k.min} onClick={() => set(n - 1)}>
              −
            </button>
            <input
              id={id}
              type="number"
              min={k.min}
              max={k.max}
              value={n}
              onChange={(e) => set(e.target.valueAsNumber)}
            />
            <button aria-label={`Increase ${o.label}`} disabled={n >= k.max} onClick={() => set(n + 1)}>
              +
            </button>
          </div>
          {help}
        </div>
      );
    }
    case "color":
      return (
        <div className="opt row" title={o.help ?? undefined}>
          <label className="opt-label" htmlFor={id}>
            {o.label}
          </label>
          <input id={id} type="color" value={value as string} onChange={(e) => onChange(e.target.value.toUpperCase())} />
          <code>{String(value)}</code>
          {help}
        </div>
      );
  }
}
