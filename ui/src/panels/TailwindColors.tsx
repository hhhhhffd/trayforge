import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { copyText } from "../api";
import { formatColor, type ColorFormat } from "../colors";
import { useToast } from "../toast";
import { SHADES, TAILWIND_COLORS } from "./tailwindData";

const FORMATS: ColorFormat[] = ["HEX", "RGB", "HSL"];

export default function TailwindColors() {
  const [filter, setFilter] = useState("");
  const [format, setFormat] = useState<ColorFormat>("HEX");
  const toast = useToast();

  const rows = useMemo(() => {
    const q = filter.trim().toLowerCase().replace(/^#/, "");
    if (!q) return TAILWIND_COLORS;
    return TAILWIND_COLORS.filter(
      (c) => c.name.toLowerCase().includes(q) || c.shades.some((h) => h.slice(1).includes(q)),
    );
  }, [filter]);

  const copy = async (hex: string, label: string) => {
    const value = formatColor(hex, format);
    try {
      await copyText(value);
      toast(`Copied ${label}: ${value}`);
    } catch (e) {
      toast(`Copy failed: ${String(e)}`);
    }
  };

  return (
    <div className="tw">
      <div className="tw-toolbar">
        <div className="search wide">
          <Search size={15} aria-hidden="true" />
          <input
            value={filter}
            placeholder="Filter colors..."
            aria-label="Filter colors"
            spellCheck={false}
            onChange={(e) => setFilter(e.target.value)}
          />
        </div>
        <div className="seg" role="radiogroup" aria-label="Color format">
          {FORMATS.map((f) => (
            <button key={f} role="radio" aria-checked={format === f} className={format === f ? "on" : ""} onClick={() => setFormat(f)}>
              {f}
            </button>
          ))}
        </div>
      </div>
      <div className="tw-table scroll">
        <div className="tw-head">
          <span>Color</span>
          {SHADES.map((s) => (
            <span key={s}>{s}</span>
          ))}
        </div>
        {rows.map((c) => (
          <div className="tw-row" key={c.name}>
            <span className="tw-name">{c.name}</span>
            {c.shades.map((hex, i) => {
              const label = `${c.name} ${SHADES[i]}`;
              return (
                <button
                  key={hex}
                  className="tw-swatch"
                  style={{ background: hex }}
                  title={`${label}: ${formatColor(hex, format)}`}
                  aria-label={`${label}, copy ${formatColor(hex, format)}`}
                  onClick={() => void copy(hex, label)}
                />
              );
            })}
          </div>
        ))}
        {rows.length === 0 && <div className="empty-state">No colors match “{filter}”.</div>}
      </div>
    </div>
  );
}
