import { lazy, Suspense, type CSSProperties, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { Dashboard } from "./components/Dashboard";
import { QuickSwitch } from "./components/QuickSwitch";
import { Sidebar } from "./components/Sidebar";
import { Splitter } from "./components/Splitter";
import { ToolPage } from "./components/ToolPage";
import { SCREEN_COLOR_PICKER_ID } from "./lib";
import { emitAction } from "./actions";
import { flushDrafts, readDraft, writeDraft } from "./drafts";
import { useSettings } from "./settings";
import { CATEGORIES, SIDEBAR_WIDTH, type Category, type ToolInfo } from "./types";

const SettingsView = lazy(() => import("./components/SettingsView"));

export type View = { kind: "dashboard" } | { kind: "settings" } | { kind: "tool"; id: string };

export function App({ tools }: { tools: ToolInfo[] }) {
  const { settings, update, flush } = useSettings();
  const toolById = useMemo(() => new Map(tools.map((t) => [t.id, t])), [tools]);

  const [view, setView] = useState<View>(() => {
    // Set by the shell when the window is created from the tray "Settings" item.
    if ((window as { __TRAYFORGE_INITIAL_ROUTE__?: string }).__TRAYFORGE_INITIAL_ROUTE__ === "settings") {
      return { kind: "settings" };
    }
    // Reopened while the app keeps running: back where the window was closed.
    const open = readDraft<View | null>("app:view", null);
    if (open && (open.kind !== "tool" || toolById.has(open.id))) return open;
    const last = settings.lastTool;
    return !settings.openDashboardOnLaunch && last && toolById.has(last)
      ? { kind: "tool", id: last }
      : { kind: "dashboard" };
  });
  useEffect(() => writeDraft("app:view", view), [view]);
  const [quickOpen, setQuickOpen] = useState(false);
  const [closing, setClosing] = useState(false);
  const [categoryFilter, setCategoryFilter] = useState<Category | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  // Input carried across tool switches when "Preserve Input on Tool Switch" is on; null
  // until a tool has been shown in this window, so the tool's own draft is used instead.
  const carriedInput = useRef<string | null>(null);

  const openTool = useCallback(
    (id: string) => {
      if (!toolById.has(id)) return;
      const recent = [id, ...settings.recent.filter((r) => r !== id)].slice(0, settings.recentLimit);
      update({ recent, lastTool: id });
      setView({ kind: "tool", id });
      setQuickOpen(false);
    },
    [settings.recent, toolById, update],
  );

  const currentToolId = view.kind === "tool" ? view.id : null;

  const toggleFavorite = useCallback(
    (id: string) => {
      const favorites = settings.favorites.includes(id)
        ? settings.favorites.filter((f) => f !== id)
        : [...settings.favorites, id];
      update({ favorites });
    },
    [settings.favorites, update],
  );

  const hide = useCallback(async () => {
    await flush();
    await api.hideWindow().catch((e) => console.error("hide_window failed", e));
  }, [flush]);

  useEffect(() => {
    const unlisten = api.onNavigate((route) => {
      if (route === "settings") setView({ kind: "settings" });
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const unlisten = api.onClosing(() => {
      setClosing(true);
      void flush();
      void flushDrafts();
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [flush]);

  // Frameless window: headers marked with data-drag move it, except over their controls.
  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (e.button !== 0 || e.detail > 1 || !(e.target instanceof Element)) return;
      if (!e.target.closest("[data-drag]")) return;
      if (e.target.closest("button, input, textarea, select, a, [role=separator], [contenteditable]")) return;
      e.preventDefault();
      api.startDragging().catch((err) => console.error("startDragging failed", err));
    };
    window.addEventListener("mousedown", onDown);
    return () => window.removeEventListener("mousedown", onDown);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        if (quickOpen) setQuickOpen(false);
        else void hide();
        return;
      }
      if (!e.ctrlKey || e.altKey || e.metaKey) return;
      const key = e.key.toLowerCase();
      let handled = true;
      if (!e.shiftKey && key === "k") {
        searchRef.current?.focus();
        searchRef.current?.select();
      } else if (!e.shiftKey && key === "p") setQuickOpen((o) => !o);
      else if (e.shiftKey && key === "p") openTool(SCREEN_COLOR_PICKER_ID);
      else if (!e.shiftKey && e.key === "Enter") emitAction("process");
      else if (e.shiftKey && key === "c") emitAction("copy");
      else if (!e.shiftKey && key === "l") emitAction("clear");
      else if (e.shiftKey && key === "s") emitAction("swap");
      else if (!e.shiftKey && key === "b") {
        if (currentToolId) toggleFavorite(currentToolId);
      } else if (!e.shiftKey && key === ",") setView({ kind: "settings" });
      else if (!e.shiftKey && /^[1-7]$/.test(key)) {
        const category = CATEGORIES[Number(key) - 1];
        setCategoryFilter((c) => (c === category ? null : category));
      } else handled = false;
      if (handled) e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [quickOpen, hide, openTool, currentToolId, toggleFavorite]);

  const tool = currentToolId ? toolById.get(currentToolId) : undefined;

  return (
    <div className={closing ? "app closing" : "app"} style={{ "--sidebar-w": `${settings.sidebarWidth}px` } as CSSProperties}>
      <Sidebar
        tools={tools}
        view={view}
        searchRef={searchRef}
        categoryFilter={categoryFilter}
        onClearFilter={() => setCategoryFilter(null)}
        onDashboard={() => setView({ kind: "dashboard" })}
        onOpenTool={openTool}
        onQuickSwitch={() => setQuickOpen(true)}
        onSettings={() => setView({ kind: "settings" })}
      />
      <Splitter
        className="side-split"
        orientation="vertical"
        label="Resize sidebar"
        value={settings.sidebarWidth}
        min={SIDEBAR_WIDTH.min}
        max={SIDEBAR_WIDTH.max}
        defaultValue={SIDEBAR_WIDTH.def}
        onChange={(sidebarWidth) => update({ sidebarWidth })}
      />
      <main className="main">
        {view.kind === "dashboard" && <Dashboard tools={tools} onOpenTool={openTool} />}
        {view.kind === "settings" && (
          <Suspense fallback={<div className="loading">Loading…</div>}>
            <SettingsView tools={tools} />
          </Suspense>
        )}
        {tool && (
          <ToolPage
            key={tool.id}
            tool={tool}
            isFavorite={settings.favorites.includes(tool.id)}
            onToggleFavorite={() => toggleFavorite(tool.id)}
            carriedInput={carriedInput}
          />
        )}
      </main>
      {quickOpen && (
        <QuickSwitch tools={tools} onClose={() => setQuickOpen(false)} onOpenTool={openTool} />
      )}
    </div>
  );
}
