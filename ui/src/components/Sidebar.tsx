import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { ChevronDown, ChevronRight, LayoutGrid, Search, Settings, X } from "lucide-react";
import type { View } from "../App";
import { categoryTools, CATEGORY_INFO, searchTools } from "../lib";
import { useSettings } from "../settings";
import type { Category, ToolInfo } from "../types";
import { Icon } from "./Icon";

interface Props {
  tools: ToolInfo[];
  view: View;
  searchRef: RefObject<HTMLInputElement>;
  categoryFilter: Category | null;
  onClearFilter: () => void;
  onDashboard: () => void;
  onOpenTool: (id: string) => void;
  onQuickSwitch: () => void;
  onSettings: () => void;
}

export function Sidebar(p: Props) {
  const { settings, update } = useSettings();
  const [query, setQuery] = useState("");
  const [recentCollapsed, setRecentCollapsed] = useState(false);
  const activeId = p.view.kind === "tool" ? p.view.id : null;
  const byId = useMemo(() => new Map(p.tools.map((t) => [t.id, t])), [p.tools]);

  const results = useMemo(() => (query.trim() ? searchTools(p.tools, query) : null), [p.tools, query]);
  // Clicking a Recent row moves it to the top of the list. With the pointer inside the list the browser's
  // scroll anchoring then follows the moved row and yanks the list to the top, so the order shown is frozen
  // while hovered and caught up on pointer leave (the stored order is always current).
  const hovered = useRef(false);
  const [shownRecent, setShownRecent] = useState(settings.recent);
  useEffect(() => {
    if (!hovered.current) setShownRecent(settings.recent);
  }, [settings.recent]);
  const recent = shownRecent
    .filter((id) => settings.recent.slice(0, settings.recentLimit).includes(id))
    .map((id) => byId.get(id))
    .filter((t): t is ToolInfo => !!t);
  const favorites = settings.favorites.map((id) => byId.get(id)).filter((t): t is ToolInfo => !!t);
  const visibleCategories = settings.categoryOrder.filter(
    (c) =>
      !settings.hiddenCategories.includes(c) &&
      (!p.categoryFilter || c === p.categoryFilter) &&
      p.tools.some((t) => t.category === c),
  );

  const open = (id: string) => {
    setQuery("");
    p.onOpenTool(id);
  };

  const toggleCollapsed = (c: Category) =>
    update({
      collapsedCategories: settings.collapsedCategories.includes(c)
        ? settings.collapsedCategories.filter((x) => x !== c)
        : [...settings.collapsedCategories, c],
    });

  const row = (t: ToolInfo, keyPrefix: string) => (
    <button
      key={`${keyPrefix}-${t.id}`}
      className={`side-row${t.id === activeId ? " active" : ""}`}
      onClick={() => open(t.id)}
      title={t.description}
    >
      <Icon name={t.icon} size={16} />
      <span className="side-row-name">{t.name}</span>
    </button>
  );

  return (
    <aside className="sidebar">
      <div className="side-top" data-drag>
        <div className="search">
          <Search size={15} aria-hidden="true" />
          <input
            ref={p.searchRef}
            value={query}
            placeholder="Search tools..."
            aria-label="Search tools"
            spellCheck={false}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && results?.[0]) open(results[0].id);
              else if (e.key === "Escape" && query) {
                e.stopPropagation();
                setQuery("");
              }
            }}
          />
          <kbd title="Search tools (Ctrl+K)">Ctrl K</kbd>
        </div>
        <button
          className={`side-row dash-link${p.view.kind === "dashboard" ? " active" : ""}`}
          onClick={p.onDashboard}
        >
          <LayoutGrid size={16} aria-hidden="true" />
          <span className="side-row-name">Dashboard</span>
        </button>
      </div>

      <nav
        className="side-list"
        aria-label="Tools"
        onPointerEnter={() => (hovered.current = true)}
        onPointerLeave={() => {
          hovered.current = false;
          setShownRecent(settings.recent);
        }}
      >
        {results ? (
          results.length ? (
            results.map((t) => row(t, "q"))
          ) : (
            <div className="side-empty">No tools found</div>
          )
        ) : (
          <>
            {p.categoryFilter && (
              <button className="filter-chip" onClick={p.onClearFilter} title="Show all categories">
                {p.categoryFilter}
                <X size={12} aria-hidden="true" />
              </button>
            )}
            {settings.showRecent && settings.recentLimit > 0 && recent.length > 0 && !p.categoryFilter && (
              <section>
                <button className="side-heading" onClick={() => setRecentCollapsed((c) => !c)}>
                  Recent
                </button>
                {!recentCollapsed && recent.map((t) => row(t, "r"))}
              </section>
            )}
            {favorites.length > 0 && !p.categoryFilter && (
              <section>
                <div className="side-heading static">Favorites</div>
                {favorites.map((t) => row(t, "f"))}
              </section>
            )}
            {visibleCategories.map((c) => {
              const collapsed = settings.collapsedCategories.includes(c) && !p.categoryFilter;
              return (
                <section key={c}>
                  <button
                    className="side-heading upper"
                    aria-expanded={!collapsed}
                    onClick={() => toggleCollapsed(c)}
                    title={CATEGORY_INFO[c].blurb}
                  >
                    {collapsed ? <ChevronRight size={13} /> : <ChevronDown size={13} />}
                    {c}
                  </button>
                  {!collapsed &&
                    categoryTools(p.tools, c, settings.toolOrder[c]).map((t) => row(t, "c"))}
                </section>
              );
            })}
          </>
        )}
      </nav>

      <div className="side-bottom">
        <button className="quick" onClick={p.onQuickSwitch} title="Quick Switch (Ctrl+P)">
          <Search size={15} aria-hidden="true" />
          <span>Quick Switch</span>
          <kbd>Ctrl P</kbd>
        </button>
        <span className="vsep" />
        <button
          className={`icon-btn${p.view.kind === "settings" ? " active" : ""}`}
          onClick={p.onSettings}
          title="Settings (Ctrl+,)"
          aria-label="Settings"
        >
          <Settings size={17} />
        </button>
      </div>
    </aside>
  );
}
