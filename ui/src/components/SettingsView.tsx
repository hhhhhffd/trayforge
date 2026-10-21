import { useEffect, useState, type ReactNode } from "react";
import {
  ChevronDown,
  ChevronRight,
  ChevronUp,
  Columns3,
  FileText,
  GripVertical,
  Info,
  Palette,
  Keyboard,
  PanelLeft,
  RotateCcw,
  Settings as SettingsIcon,
  Sparkles,
  SquareDashed,
} from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { api } from "../api";
import { acceleratorFromEvent, categoryTools, CATEGORY_INFO, moveItem } from "../lib";
import { useSettings } from "../settings";
import { useToast } from "../toast";
import { LICENSE_GROUPS } from "../licenses";
import {
  ACCENTS,
  CATEGORIES,
  DEFAULT_HOTKEY,
  GLASS_OPACITY,
  RECENT_LIMIT,
  WINDOW_EFFECTS,
  type AccentColor,
  type Category,
  type Theme,
  type ToolInfo,
} from "../types";
import { ConfirmDialog } from "./ConfirmDialog";
import { Icon } from "./Icon";

const REPO_URL = "https://github.com/hhhhhffd/trayforge";

const TABS = [
  { id: "General", icon: SettingsIcon },
  { id: "Appearance", icon: Sparkles },
  { id: "Sidebar", icon: PanelLeft },
  { id: "Keyboard", icon: Keyboard },
  { id: "About", icon: Info },
] as const;
type Tab = (typeof TABS)[number]["id"];

// In-app shortcuts (see docs/ARCHITECTURE.md).
const NAVIGATION_SHORTCUTS: [string, string, string][] = [
  ["Search Tools", "Focus the search field", "Ctrl+K"],
  ["Quick Switch", "Open the quick tool switcher", "Ctrl+P"],
  ["Process / Convert", "Run the current tool", "Ctrl+Enter"],
  ["Copy Output", "Copy result to clipboard", "Ctrl+Shift+C"],
  ["Clear All", "Clear input and output", "Ctrl+L"],
  ["Swap Input/Output", "Swap for bidirectional tools", "Ctrl+Shift+S"],
  ["Toggle Favorite", "Add or remove from favorites", "Ctrl+B"],
  ["Color Picker", "Open the screen color picker", "Ctrl+Shift+P"],
  ["Settings", "Open settings", "Ctrl+,"],
  ["Close", "Close the window when no overlay is open", "Esc"],
];

function Section({ icon, title, children }: { icon: ReactNode; title: string; children: ReactNode }) {
  return (
    <section className="set-section">
      <h3>
        {icon} {title}
      </h3>
      {children}
    </section>
  );
}

function Row({ title, desc, children }: { title: string; desc?: string; children: ReactNode }) {
  return (
    <div className="set-row">
      <div className="set-text">
        <strong>{title}</strong>
        {desc && <small>{desc}</small>}
      </div>
      {children}
    </div>
  );
}

function Switch({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button role="switch" aria-checked={checked} aria-label={label} className={`switch${checked ? " on" : ""}`} onClick={() => onChange(!checked)}>
      <span />
    </button>
  );
}

export default function SettingsView({ tools }: { tools: ToolInfo[] }) {
  const [tab, setTab] = useState<Tab>("General");
  return (
    <div className="settings">
      <div className="set-tabs" role="tablist" data-drag>
        {TABS.map(({ id, icon: TabIcon }) => (
          <button key={id} role="tab" aria-selected={tab === id} className={tab === id ? "on" : ""} onClick={() => setTab(id)}>
            <TabIcon size={22} aria-hidden="true" />
            {id}
          </button>
        ))}
      </div>
      <div className="set-body scroll">
        {tab === "General" && <General />}
        {tab === "Appearance" && <Appearance />}
        {tab === "Sidebar" && <SidebarSettings tools={tools} />}
        {tab === "Keyboard" && <KeyboardSettings />}
        {tab === "About" && <About />}
      </div>
    </div>
  );
}

function General() {
  const { settings, update, reset } = useSettings();
  const toast = useToast();
  const [confirmReset, setConfirmReset] = useState(false);

  const setLaunchAtLogin = async (enabled: boolean) => {
    try {
      await api.setLaunchAtLogin(enabled);
      update({ launchAtLogin: enabled });
    } catch (e) {
      toast(`Could not change launch at login: ${String(e)}`);
    }
  };

  const doReset = async () => {
    setConfirmReset(false);
    if (settings.launchAtLogin) {
      try {
        await api.setLaunchAtLogin(false);
      } catch (e) {
        toast(`Could not disable launch at login: ${String(e)}`);
      }
    }
    reset();
    toast("Preferences reset");
  };

  const n = settings.recent.length;
  return (
    <>
      <Section icon={<RotateCcw size={16} />} title="STARTUP">
        <div className="set-card">
          <Row title="Open Dashboard on Launch" desc="Show the Dashboard when Trayforge opens instead of resuming the last used tool">
            <Switch label="Open Dashboard on Launch" checked={settings.openDashboardOnLaunch} onChange={(v) => update({ openDashboardOnLaunch: v })} />
          </Row>
          <Row title="Launch at Login" desc="Automatically start Trayforge when you log in to Windows">
            <Switch label="Launch at Login" checked={settings.launchAtLogin} onChange={(v) => void setLaunchAtLogin(v)} />
          </Row>
        </div>
      </Section>
      <Section icon={<Columns3 size={16} />} title="WINDOW MODE">
        <div className="set-card">
          <Row title="Standalone Window" desc="Open Trayforge as a standalone window instead of a tray popover. The tray icon remains available. Requires restart.">
            <Switch label="Standalone Window" checked={settings.standaloneWindow} onChange={(v) => update({ standaloneWindow: v })} />
          </Row>
        </div>
      </Section>
      <Section icon={<SettingsIcon size={16} />} title="BEHAVIOR">
        <div className="set-card">
          <Row title="Preserve Input on Tool Switch" desc="Keep the current input text when switching to a different tool">
            <Switch label="Preserve Input on Tool Switch" checked={settings.preserveInput} onChange={(v) => update({ preserveInput: v })} />
          </Row>
        </div>
      </Section>
      <Section icon={<FileText size={16} />} title="DATA">
        <div className="set-card">
          <Row title="Recent Tools" desc={`${n} tool${n === 1 ? "" : "s"} in history`}>
            <button className="soft" disabled={n === 0} onClick={() => update({ recent: [] })}>
              Clear
            </button>
          </Row>
          <Row title="Reset All Preferences" desc="Restore all settings to their defaults">
            <button className="soft" onClick={() => setConfirmReset(true)}>
              Reset...
            </button>
          </Row>
        </div>
      </Section>
      {confirmReset && (
        <ConfirmDialog
          title="Reset All Preferences?"
          message="All settings, favorites, recent tools and saved tool options return to their defaults. This cannot be undone."
          confirmLabel="Reset"
          onConfirm={() => void doReset()}
          onCancel={() => setConfirmReset(false)}
        />
      )}
    </>
  );
}

function Appearance() {
  const { settings, update } = useSettings();
  const toast = useToast();
  const resetWindow = () =>
    api.resetWindowState().catch((e) => toast(`Could not reset the window: ${String(e)}`));
  const size = (key: "popoverWidth" | "popoverHeight", label: string, min: number, max: number) => (
    <Row title={label} desc={undefined}>
      <span className="size-input">
        <input
          type="number"
          min={min}
          max={max}
          value={settings[key]}
          aria-label={label}
          onChange={(e) => {
            const v = e.target.valueAsNumber;
            if (Number.isFinite(v)) update({ [key]: Math.min(max, Math.max(min, Math.round(v))) });
          }}
        />
        <span className="muted">px</span>
      </span>
    </Row>
  );
  const glassOff = settings.appearance === "classic" || settings.windowEffect === "none";
  return (
    <>
      <Section icon={<Palette size={16} />} title="THEME">
        <div className="set-card">
          <Row title="Theme" desc="System follows the Windows app mode">
            <div className="seg" role="radiogroup" aria-label="Theme">
              {(["system", "dark", "light"] as Theme[]).map((t) => (
                <button key={t} role="radio" aria-checked={settings.theme === t} className={settings.theme === t ? "on" : ""} onClick={() => update({ theme: t })}>
                  {t[0].toUpperCase() + t.slice(1)}
                </button>
              ))}
            </div>
          </Row>
          <Row title="Accent Color">
            <div className="swatches" role="radiogroup" aria-label="Accent color">
              {(Object.keys(ACCENTS) as AccentColor[]).map((id) => (
                <button
                  key={id}
                  role="radio"
                  aria-checked={settings.accentColor === id}
                  aria-label={ACCENTS[id].label}
                  title={ACCENTS[id].label}
                  className={`swatch${settings.accentColor === id ? " on" : ""}`}
                  style={{ background: ACCENTS[id].base }}
                  onClick={() => update({ accentColor: id })}
                />
              ))}
            </div>
          </Row>
        </div>
      </Section>
      <Section icon={<Sparkles size={16} />} title="VISUAL STYLE">
        <div className="set-card">
          <div className="style-cards" role="radiogroup" aria-label="Visual style">
            {(
              [
                ["classic", "Classic", "Clean flat surfaces"],
                ["glass", "Liquid Glass", "Translucent blur layers"],
              ] as const
            ).map(([id, name, desc]) => (
              <button key={id} role="radio" aria-checked={settings.appearance === id} className={`style-card${settings.appearance === id ? " on" : ""}`} onClick={() => update({ appearance: id })}>
                <span className={`style-preview ${id}`} />
                <strong>{name}</strong>
                <small>{desc}</small>
              </button>
            ))}
          </div>
          <p className="set-note">
            <Info size={15} aria-hidden="true" /> Sidebar, dashboard cards, and panels become translucent. The app window blurs
            content behind it.
          </p>
        </div>
      </Section>
      <Section icon={<Sparkles size={16} />} title="LIQUID GLASS">
        <div className="set-card">
          <Row title="Window Effect" desc="Applies next time the window opens">
            <select aria-label="Window effect" value={settings.windowEffect} onChange={(e) => update({ windowEffect: e.target.value as typeof settings.windowEffect })}>
              {WINDOW_EFFECTS.map((o) => (
                <option key={o.id} value={o.id}>
                  {o.label}
                </option>
              ))}
            </select>
          </Row>
          <Row title="Glass Opacity" desc={glassOff ? "Surfaces are opaque with the Classic style or the None effect" : `${Math.round(settings.glassOpacity * 100)}% surface opacity`}>
            <input
              type="range"
              aria-label="Glass opacity"
              min={GLASS_OPACITY.min}
              max={GLASS_OPACITY.max}
              step={0.01}
              disabled={glassOff}
              value={settings.glassOpacity}
              onChange={(e) => update({ glassOpacity: Number(e.target.value) })}
            />
          </Row>
        </div>
      </Section>
      <Section icon={<SquareDashed size={16} />} title="WINDOW SIZE">
        <div className="set-card">
          {size("popoverWidth", "Width", 600, 2000)}
          {size("popoverHeight", "Height", 400, 1600)}
          <p className="set-note muted">Changes will take effect the next time you open the window. Recommended: 750 × 650.</p>
          <Row title="Resized Size" desc="Resize the window from its edges; it reopens next to the taskbar at the size you left it">
            <button className="soft" onClick={() => void resetWindow()}>
              Reset
            </button>
          </Row>
        </div>
      </Section>
    </>
  );
}

function SidebarSettings({ tools }: { tools: ToolInfo[] }) {
  const { settings, update } = useSettings();
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  const [openOrder, setOpenOrder] = useState<Category | null>(null);

  const toggleHidden = (c: Category) =>
    update({
      hiddenCategories: settings.hiddenCategories.includes(c)
        ? settings.hiddenCategories.filter((x) => x !== c)
        : [...settings.hiddenCategories, c],
    });

  // Lowering the limit also trims the stored history.
  const setRecentLimit = (n: number) => {
    if (!Number.isFinite(n)) return;
    const recentLimit = Math.min(RECENT_LIMIT.max, Math.max(RECENT_LIMIT.min, Math.trunc(n)));
    update({ recentLimit, recent: settings.recent.slice(0, recentLimit) });
  };

  const moveTool = (c: Category, ids: string[], index: number, delta: number) =>
    update({ toolOrder: { ...settings.toolOrder, [c]: moveItem(ids, index, delta) } });

  return (
    <>
      <Section icon={<RotateCcw size={16} />} title="SECTIONS">
        <div className="set-card">
          <Row title="Show Recent Tools" desc="Display your recently used tools at the top of the sidebar">
            <Switch label="Show Recent Tools" checked={settings.showRecent} onChange={(v) => update({ showRecent: v })} />
          </Row>
          <Row title="Recent Tools Limit" desc="How many recent tools to keep (0 hides Recent)">
            <div className="stepper">
              <button aria-label="Decrease limit" disabled={settings.recentLimit <= RECENT_LIMIT.min} onClick={() => setRecentLimit(settings.recentLimit - 1)}>
                −
              </button>
              <input
                type="number"
                aria-label="Recent tools limit"
                min={RECENT_LIMIT.min}
                max={RECENT_LIMIT.max}
                value={settings.recentLimit}
                onChange={(e) => setRecentLimit(e.target.valueAsNumber)}
              />
              <button aria-label="Increase limit" disabled={settings.recentLimit >= RECENT_LIMIT.max} onClick={() => setRecentLimit(settings.recentLimit + 1)}>
                +
              </button>
            </div>
          </Row>
        </div>
      </Section>
      <Section icon={<PanelLeft size={16} />} title="CATEGORIES">
        <p className="set-note muted">Drag rows or use the arrows to reorder. Use the toggle to show or hide a category.</p>
        <div className="set-card">
          {settings.categoryOrder.map((c, i) => (
            <div
              key={c}
              className="cat-row"
              draggable
              onDragStart={() => setDragIndex(i)}
              onDragOver={(e) => e.preventDefault()}
              onDrop={() => {
                if (dragIndex !== null && dragIndex !== i) {
                  const next = [...settings.categoryOrder];
                  next.splice(i, 0, next.splice(dragIndex, 1)[0]);
                  update({ categoryOrder: next });
                }
                setDragIndex(null);
              }}
            >
              <GripVertical size={16} className="muted" aria-hidden="true" />
              <Icon name={CATEGORY_INFO[c].icon} size={20} className="accent" />
              <div className="set-text">
                <strong>{c}</strong>
                <small>{CATEGORY_INFO[c].blurb}</small>
              </div>
              <button className="icon-btn" aria-label={`Move ${c} up`} disabled={i === 0} onClick={() => update({ categoryOrder: moveItem(settings.categoryOrder, i, -1) })}>
                <ChevronUp size={16} />
              </button>
              <button className="icon-btn" aria-label={`Move ${c} down`} disabled={i === settings.categoryOrder.length - 1} onClick={() => update({ categoryOrder: moveItem(settings.categoryOrder, i, 1) })}>
                <ChevronDown size={16} />
              </button>
              <Switch label={`Show ${c}`} checked={!settings.hiddenCategories.includes(c)} onChange={() => toggleHidden(c)} />
            </div>
          ))}
        </div>
      </Section>
      <Section icon={<FileText size={16} />} title="TOOL ORDER">
        <div className="set-card">
          {CATEGORIES.map((c) => {
            const ordered = categoryTools(tools, c, settings.toolOrder[c]);
            const ids = ordered.map((t) => t.id);
            const open = openOrder === c;
            return (
              <div key={c}>
                <button className="order-head" aria-expanded={open} onClick={() => setOpenOrder(open ? null : c)}>
                  {open ? <ChevronDown size={15} /> : <ChevronRight size={15} />} {c}
                  <span className="muted">{ordered.length}</span>
                </button>
                {open &&
                  ordered.map((t, i) => (
                    <div className="order-row" key={t.id}>
                      <Icon name={t.icon} size={16} />
                      <span className="grow">{t.name}</span>
                      <button className="icon-btn" aria-label={`Move ${t.name} up`} disabled={i === 0} onClick={() => moveTool(c, ids, i, -1)}>
                        <ChevronUp size={15} />
                      </button>
                      <button className="icon-btn" aria-label={`Move ${t.name} down`} disabled={i === ordered.length - 1} onClick={() => moveTool(c, ids, i, 1)}>
                        <ChevronDown size={15} />
                      </button>
                    </div>
                  ))}
              </div>
            );
          })}
        </div>
      </Section>
      <div className="set-actions">
        <button className="soft" onClick={() => update({ collapsedCategories: [...CATEGORIES] })}>
          Collapse All
        </button>
        <button className="soft" onClick={() => update({ collapsedCategories: [] })}>
          Expand All
        </button>
      </div>
    </>
  );
}

function KeyboardSettings() {
  const { settings, update } = useSettings();
  const toast = useToast();
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    // Capture phase + stopImmediatePropagation keeps App's shortcuts (Esc, Ctrl+P...) from firing.
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (e.key === "Escape") return setRecording(false);
      const accel = acceleratorFromEvent(e);
      if (!accel) return;
      if (!(e.ctrlKey || e.altKey || e.metaKey)) {
        toast("A global hotkey needs Ctrl, Alt or Win");
        return;
      }
      update({ globalHotkey: accel });
      setRecording(false);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, update, toast]);

  return (
    <>
      <Section icon={<Keyboard size={16} />} title="GLOBAL">
        <div className="set-card">
          <Row title="Toggle Trayforge" desc="Show or hide Trayforge from anywhere">
            <span className="hotkey">
              <button className={`kbd-btn${recording ? " recording" : ""}`} onClick={() => setRecording((r) => !r)} title="Click, then press the new shortcut (Esc cancels)">
                {recording ? "Press shortcut…" : settings.globalHotkey}
              </button>
              <button className="icon-btn" aria-label="Reset to default" title="Reset to default" disabled={settings.globalHotkey === DEFAULT_HOTKEY} onClick={() => update({ globalHotkey: DEFAULT_HOTKEY })}>
                <RotateCcw size={16} />
              </button>
            </span>
          </Row>
        </div>
      </Section>
      <Section icon={<Keyboard size={16} />} title="NAVIGATION">
        <div className="set-card">
          {NAVIGATION_SHORTCUTS.map(([name, desc, keys]) => (
            <Row key={name} title={name} desc={desc}>
              <kbd className="kbd">{keys}</kbd>
            </Row>
          ))}
        </div>
      </Section>
      <Section icon={<Keyboard size={16} />} title="CATEGORY SHORTCUTS">
        <div className="set-card">
          {CATEGORIES.map((c, i) => (
            <Row key={c} title={c} desc={`Filter to ${c} tools`}>
              <kbd className="kbd">Ctrl+{i + 1}</kbd>
            </Row>
          ))}
        </div>
      </Section>
    </>
  );
}

function About() {
  const [version, setVersion] = useState("dev");
  useEffect(() => {
    // Outside Tauri (browser dev mock) there is no app metadata; keep "dev".
    if (!("__TAURI_INTERNALS__" in window)) return;
    getVersion().then(setVersion, (e) => console.error("getVersion failed", e));
  }, []);
  return (
    <div className="about">
      <h2>Trayforge</h2>
      <p className="muted">Open source developer toolbox for Windows</p>
      <p className="mono muted">Version {version}</p>
      <p className="muted">
        <a
          href={REPO_URL}
          onClick={(e) => {
            e.preventDefault();
            api.openExternal(REPO_URL).catch((err) => console.error("openUrl failed", err));
          }}
        >
          github.com/hhhhhffd/trayforge
        </a>
        {" · MIT License · © 2026 hhhhhffd"}
      </p>
      <Section icon={<FileText size={16} />} title="OPEN SOURCE LICENSES">
        {LICENSE_GROUPS.map((g) => (
          <div key={g.title}>
            <p className="set-note muted">{g.title}</p>
            <div className="set-card">
              {g.entries.map((d) => (
                <Row key={d.name} title={d.name} desc={d.version}>
                  <code className="tag">{d.license}</code>
                </Row>
              ))}
            </div>
          </div>
        ))}
      </Section>
    </div>
  );
}
