import { copyText } from "../api";
import { useToast } from "../toast";

// Built-in palettes for the Color Palettes tool.
const PALETTES: { name: string; colors: string[] }[] = [
  { name: "Monochrome", colors: ["#0F0F0F", "#373737", "#6E6E6E", "#B4B4B4", "#F0F0F0"] },
  { name: "Forest Walk", colors: ["#22573C", "#3F8148", "#78B463", "#A8C77C", "#D2E0B4"] },
  { name: "Ocean Breeze", colors: ["#0079BF", "#00A9CE", "#48CAE4", "#90E0EF", "#CAF0F8"] },
  { name: "Warm Sunset", colors: ["#FF5E4D", "#FF8F59", "#FFC857", "#DC5B56", "#8E44AD"] },
  { name: "Pastel Dream", colors: ["#FFB3BA", "#FFDFBA", "#FFFFBA", "#BAFFC9", "#BAE1FF"] },
  { name: "Deep Space", colors: ["#0B0C1E", "#1A1A3E", "#2D2B55", "#4A4580", "#7B76BF"] },
  { name: "Neon Nights", colors: ["#E8336D", "#E8622C", "#F5C242", "#4A82F4", "#7C3AE0"] },
  { name: "Earth Tones", colors: ["#7F5A3F", "#9A7B58", "#C3AA85", "#D8C7A2", "#EBE7D8"] },
  { name: "Berry Smoothie", colors: ["#4A1942", "#893168", "#C44569", "#F78FB3", "#FDCBDC"] },
  { name: "Arctic Frost", colors: ["#1B3A4B", "#3D6B80", "#7AA6B8", "#B9D6E0", "#EAF4F8"] },
  { name: "Autumn Leaves", colors: ["#5C2A1E", "#A0442A", "#D9772B", "#E9B44C", "#F4DDA0"] },
  { name: "Retro Pop", colors: ["#264653", "#2A9D8F", "#E9C46A", "#F4A261", "#E76F51"] },
];

export default function ColorPalettes() {
  const toast = useToast();

  const copy = async (hex: string) => {
    try {
      await copyText(hex);
      toast(`Copied ${hex}`);
    } catch (e) {
      toast(`Copy failed: ${String(e)}`);
    }
  };

  return (
    <div className="palettes scroll">
      {PALETTES.map((p) => (
        <section className="palette" key={p.name} aria-label={p.name}>
          <h3>{p.name}</h3>
          <div className="palette-row">
            {p.colors.map((hex) => (
              <button key={hex} className="swatch" onClick={() => void copy(hex)} title={`Copy ${hex}`} aria-label={`Copy ${hex}`}>
                <span className="swatch-color" style={{ background: hex }} />
                <code>{hex}</code>
              </button>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
