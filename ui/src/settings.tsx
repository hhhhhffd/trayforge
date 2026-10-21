import { createContext, useCallback, useContext, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { api } from "./api";
import {
  ACCENTS,
  CATEGORIES,
  DEFAULT_SETTINGS,
  GLASS_OPACITY,
  INPUT_HEIGHT,
  OPTIONS_WIDTH,
  RECENT_LIMIT,
  SIDEBAR_WIDTH,
  WINDOW_EFFECTS,
  type Category,
  type Settings,
} from "./types";

const SAVE_DEBOUNCE_MS = 300;

function clampNumber(v: unknown, min: number, max: number, def: number): number {
  return typeof v === "number" && Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : def;
}

/** Merges the stored JSON over defaults, dropping values of the wrong shape. */
export function normalizeSettings(raw: Record<string, unknown>): Settings {
  const s: Settings = { ...DEFAULT_SETTINGS, ...(raw as Partial<Settings>) };
  const order = (Array.isArray(s.categoryOrder) ? s.categoryOrder : []).filter((c): c is Category =>
    CATEGORIES.includes(c),
  );
  s.categoryOrder = [...new Set([...order, ...CATEGORIES])];
  for (const key of ["favorites", "recent", "hiddenCategories", "collapsedCategories"] as const) {
    if (!Array.isArray(s[key])) (s[key] as unknown) = DEFAULT_SETTINGS[key];
  }
  if (typeof s.toolOrder !== "object" || s.toolOrder === null) s.toolOrder = {};
  if (typeof s.toolOptions !== "object" || s.toolOptions === null) s.toolOptions = {};
  if (s.appearance !== "classic") s.appearance = "glass";
  if (!WINDOW_EFFECTS.some((e) => e.id === s.windowEffect)) s.windowEffect = DEFAULT_SETTINGS.windowEffect;
  if (s.theme !== "dark" && s.theme !== "light") s.theme = "system";
  if (!Object.hasOwn(ACCENTS, s.accentColor)) s.accentColor = DEFAULT_SETTINGS.accentColor;
  s.glassOpacity = clampNumber(s.glassOpacity, GLASS_OPACITY.min, GLASS_OPACITY.max, GLASS_OPACITY.def);
  s.recentLimit = Math.round(clampNumber(s.recentLimit, RECENT_LIMIT.min, RECENT_LIMIT.max, RECENT_LIMIT.def));
  s.recent = s.recent.slice(0, s.recentLimit);
  s.sidebarWidth = Math.round(clampNumber(s.sidebarWidth, SIDEBAR_WIDTH.min, SIDEBAR_WIDTH.max, SIDEBAR_WIDTH.def));
  // The upper bound (half the window) depends on the live window size and is applied at render time.
  s.optionsWidth = Math.round(clampNumber(s.optionsWidth, OPTIONS_WIDTH.min, 4000, OPTIONS_WIDTH.def));
  s.inputHeight = Math.round(clampNumber(s.inputHeight, INPUT_HEIGHT.min, INPUT_HEIGHT.max, INPUT_HEIGHT.def));
  return s;
}

interface SettingsApi {
  settings: Settings;
  update: (patch: Partial<Settings>) => void;
  /** Replaces everything with defaults (the "Reset All Preferences" action). */
  reset: () => void;
  /** Writes pending changes immediately; await before the window is destroyed. */
  flush: () => Promise<void>;
}

const SettingsContext = createContext<SettingsApi | null>(null);

export function useSettings(): SettingsApi {
  const ctx = useContext(SettingsContext);
  if (!ctx) throw new Error("useSettings outside SettingsProvider");
  return ctx;
}

export function SettingsProvider({
  initial,
  rawExtras,
  children,
}: {
  initial: Settings;
  /** Stored keys this UI does not know (e.g. written by the shell); kept on save. */
  rawExtras: Record<string, unknown>;
  children: ReactNode;
}) {
  const [settings, setSettings] = useState(initial);
  const latest = useRef(initial);
  const dirty = useRef(false);
  const timer = useRef<number>();

  const flush = useCallback(async () => {
    window.clearTimeout(timer.current);
    if (!dirty.current) return;
    dirty.current = false;
    try {
      await api.saveSettings({ ...rawExtras, ...latest.current });
    } catch (e) {
      dirty.current = true;
      console.error("save_settings failed", e);
    }
  }, [rawExtras]);

  const commit = useCallback(
    (next: Settings) => {
      latest.current = next;
      dirty.current = true;
      setSettings(next);
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => void flush(), SAVE_DEBOUNCE_MS);
    },
    [flush],
  );

  const update = useCallback(
    (patch: Partial<Settings>) => commit({ ...latest.current, ...patch }),
    [commit],
  );
  const reset = useCallback(() => commit(normalizeSettings({})), [commit]);

  // The window can be destroyed at any time (hotkey, tray); write on hide.
  useEffect(() => {
    const onHide = () => {
      if (document.visibilityState === "hidden") void flush();
    };
    document.addEventListener("visibilitychange", onHide);
    window.addEventListener("pagehide", onHide);
    return () => {
      document.removeEventListener("visibilitychange", onHide);
      window.removeEventListener("pagehide", onHide);
    };
  }, [flush]);

  useLayoutEffect(() => {
    const root = document.documentElement;
    const apply = () => {
      const dark = settings.theme === "dark" || (settings.theme === "system" && !colorScheme.matches);
      root.dataset.theme = dark ? "dark" : "light";
    };
    const colorScheme = window.matchMedia("(prefers-color-scheme: light)");
    apply();
    colorScheme.addEventListener("change", apply);
    return () => colorScheme.removeEventListener("change", apply);
  }, [settings.theme]);

  useLayoutEffect(() => {
    const style = document.documentElement.style;
    document.documentElement.dataset.appearance = settings.appearance;
    style.setProperty("--accent-base", ACCENTS[settings.accentColor].base);
    // Classic and an opaque window effect keep the stylesheet's solid surfaces.
    if (settings.appearance === "classic" || settings.windowEffect === "none") {
      style.removeProperty("--op");
      style.removeProperty("--editor-op");
      document.documentElement.dataset.opaque = "";
    } else {
      const op = Math.round(settings.glassOpacity * 100);
      style.setProperty("--op", `${op}%`);
      style.setProperty("--editor-op", `${Math.min(100, op + 16)}%`);
      delete document.documentElement.dataset.opaque;
    }
  }, [settings.appearance, settings.accentColor, settings.windowEffect, settings.glassOpacity]);

  return (
    <SettingsContext.Provider value={{ settings, update, reset, flush }}>
      {children}
    </SettingsContext.Provider>
  );
}
