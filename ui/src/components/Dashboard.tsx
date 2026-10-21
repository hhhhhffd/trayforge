import { CATEGORY_INFO, categoryTools } from "../lib";
import { useSettings } from "../settings";
import type { ToolInfo } from "../types";
import { Icon } from "./Icon";

export function Dashboard({ tools, onOpenTool }: { tools: ToolInfo[]; onOpenTool: (id: string) => void }) {
  const { settings } = useSettings();
  const favorites = settings.favorites
    .map((id) => tools.find((t) => t.id === id))
    .filter((t): t is ToolInfo => !!t);

  const card = (t: ToolInfo) => (
    <button key={t.id} className="card" onClick={() => onOpenTool(t.id)}>
      <Icon name={t.icon} size={32} className="tool-icon" />
      <span className="card-name">{t.name}</span>
      <span className="card-desc">{t.description}</span>
    </button>
  );

  return (
    <div className="dashboard scroll">
      <h1 data-drag>Welcome to Trayforge</h1>
      <p className="tagline" data-drag>64 developer tools in your tray: offline, instant, private.</p>
      {favorites.length > 0 && (
        <section>
          <h2>
            <Icon name="star" size={15} /> Favorites
          </h2>
          <div className="grid">{favorites.map(card)}</div>
        </section>
      )}
      {settings.categoryOrder
        .filter((c) => !settings.hiddenCategories.includes(c) && tools.some((t) => t.category === c))
        .map((c) => (
          <section key={c}>
            <h2>
              <Icon name={CATEGORY_INFO[c].icon} size={15} /> {c}
            </h2>
            <div className="grid">{categoryTools(tools, c, settings.toolOrder[c]).map(card)}</div>
          </section>
        ))}
    </div>
  );
}
