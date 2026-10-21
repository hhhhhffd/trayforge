import { useState } from "react";
import { Pipette } from "lucide-react";
import { api, copyText, toToolFailure } from "../api";
import { formatColor } from "../colors";
import { useToast } from "../toast";

export default function ScreenColorPicker() {
  const [hex, setHex] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const toast = useToast();

  const pick = async () => {
    setBusy(true);
    setError(null);
    try {
      const picked = (await api.pickScreenColor()).toUpperCase();
      setHex(picked);
      try {
        await copyText(picked);
        toast("Hex copied to clipboard");
      } catch (e) {
        toast(`Picked ${picked}, but copying failed: ${String(e)}`);
      }
    } catch (e) {
      setError(toToolFailure(e).display);
    } finally {
      setBusy(false);
    }
  };

  const copy = async (value: string, label: string) => {
    try {
      await copyText(value);
      toast(`${label} copied to clipboard`);
    } catch (e) {
      toast(`Copy failed: ${String(e)}`);
    }
  };

  return (
    <div className="picker">
      <div className="picker-icon">
        <Pipette size={44} aria-hidden="true" />
      </div>
      <h2>Screen Color Picker</h2>
      <p>
        Sample any color from anywhere on your screen.
        <br />
        The hex value is copied to your clipboard automatically.
      </p>
      <button className="primary big" onClick={() => void pick()} disabled={busy}>
        <Pipette size={17} aria-hidden="true" /> {busy ? "Picking…" : "Pick Color from Screen"}
      </button>
      {error && (
        <div className="error-box" role="alert">
          <div>
            <strong>Error</strong>
            <p>{error}</p>
          </div>
        </div>
      )}
      {hex ? (
        <div className="picked">
          <span className="picked-swatch" style={{ background: hex }} aria-label={`Color ${hex}`} />
          <div className="picked-values">
            {(["HEX", "RGB", "HSL"] as const).map((f) => {
              const v = formatColor(hex, f);
              return (
                <button key={f} onClick={() => void copy(v, f)} title={`Copy ${f}`}>
                  <span className="tag">{f}</span>
                  <code>{v}</code>
                </button>
              );
            })}
          </div>
        </div>
      ) : (
        <div className="tags">
          <span className="tag">HEX</span>
          <span className="tag">RGB</span>
          <span className="tag">HSL</span>
          <span className="muted">copied on pick</span>
        </div>
      )}
      <p className="muted shortcut">Shortcut: Ctrl+Shift+P</p>
    </div>
  );
}
