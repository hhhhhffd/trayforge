// Wire types: mirror crates/trayforge-core/src/tool.rs and docs/ARCHITECTURE.md.

export type Category =
  | "Encoding"
  | "Hashing"
  | "Formats"
  | "Web Dev"
  | "Text"
  | "Dev Utils"
  | "Graphics";

/** Same order as `Category::ALL` in trayforge-core. */
export const CATEGORIES: Category[] = [
  "Encoding",
  "Hashing",
  "Formats",
  "Web Dev",
  "Text",
  "Dev Utils",
  "Graphics",
];

export type OptionValue = boolean | number | string;

export type OptionKind =
  | { type: "toggle"; default: boolean }
  | { type: "picker"; choices: string[]; default: number }
  | { type: "segmented"; choices: string[]; default: number }
  | { type: "slider"; min: number; max: number; default: number; step: number }
  | { type: "textField"; placeholder: string; default: string }
  | { type: "stepper"; min: number; max: number; default: number }
  | { type: "color"; default: string };

export interface ToolOption {
  id: string;
  label: string;
  help: string | null;
  kind: OptionKind;
}

export interface ToolInfo {
  id: string;
  name: string;
  category: Category;
  description: string;
  keywords: string[];
  icon: string;
  inputPlaceholder: string;
  outputPlaceholder: string;
  primaryAction: string;
  secondaryAction: string | null;
  supportsFileInput: boolean;
  allowsEmptyInput: boolean;
  customUi: boolean;
  options: ToolOption[];
}

export interface OutputItem {
  label: string;
  value: string;
  isWarning: boolean;
  isImage: boolean;
}

export type ToolOutput =
  | { type: "text"; value: string; metadata: Record<string, string> }
  | { type: "multiple"; items: OutputItem[] };

/** Rejection of `process_tool`. `display` is the user-facing string. */
export interface ToolFailure {
  kind: string;
  message?: string;
  display: string;
}

export interface HttpRequest {
  method: string;
  url: string;
  headers: [string, string][];
  body: string | null;
}

export interface HttpResponse {
  status: number;
  statusText: string;
  headers: [string, string][];
  body: string;
  durationMs: number;
  sizeBytes: number;
}

export type TotpAlgorithm = "SHA1" | "SHA256" | "SHA512";

export interface TotpParams {
  secret: string;
  algorithm: TotpAlgorithm;
  digits: number;
  period: number;
}

export interface TotpCode {
  code: string;
  remainingSeconds: number;
}

export interface TotpUri {
  account: string;
  issuer: string;
  secret: string;
  algorithm: TotpAlgorithm;
  digits: number;
  period: number;
}

export type Appearance = "glass" | "classic";
export type WindowEffect = "acrylic" | "mica" | "blur" | "none";
export type Theme = "system" | "dark" | "light";

export const WINDOW_EFFECTS: { id: WindowEffect; label: string }[] = [
  { id: "acrylic", label: "Acrylic" },
  { id: "mica", label: "Mica" },
  { id: "blur", label: "Blur" },
  { id: "none", label: "None (opaque)" },
];

/** Preset accent colors; `base` is dark enough for white text on it in both themes. */
export const ACCENTS = {
  violet: { label: "Violet", base: "#9b57ae" },
  blue: { label: "Blue", base: "#2f6bed" },
  teal: { label: "Teal", base: "#0f766e" },
  green: { label: "Green", base: "#15803d" },
  orange: { label: "Orange", base: "#c2410c" },
  pink: { label: "Pink", base: "#db2777" },
} as const;
export type AccentColor = keyof typeof ACCENTS;

/** Pane size limits (px); the OPTIONS maximum is also capped at half the window. */
export const SIDEBAR_WIDTH = { def: 240, min: 180, max: 400 };
export const OPTIONS_WIDTH = { def: 270, min: 200 };
export const INPUT_HEIGHT = { def: 190, min: 80, max: 1200 };
export const RECENT_LIMIT = { def: 10, min: 0, max: 30 };
export const GLASS_OPACITY = { def: 0.72, min: 0.3, max: 1 };

/** The single settings JSON object. The shell only reads `globalHotkey` and `windowEffect`. */
export interface Settings {
  globalHotkey: string;
  openDashboardOnLaunch: boolean;
  launchAtLogin: boolean;
  standaloneWindow: boolean;
  preserveInput: boolean;
  showRecent: boolean;
  appearance: Appearance;
  windowEffect: WindowEffect;
  glassOpacity: number;
  theme: Theme;
  accentColor: AccentColor;
  recentLimit: number;
  sidebarWidth: number;
  optionsWidth: number;
  inputHeight: number;
  popoverWidth: number;
  popoverHeight: number;
  favorites: string[];
  recent: string[];
  lastTool: string | null;
  categoryOrder: Category[];
  hiddenCategories: Category[];
  collapsedCategories: Category[];
  toolOrder: Partial<Record<Category, string[]>>;
  toolOptions: Record<string, Record<string, OptionValue>>;
}

export const DEFAULT_HOTKEY = "Ctrl+Alt+D";

export const DEFAULT_SETTINGS: Settings = {
  globalHotkey: DEFAULT_HOTKEY,
  openDashboardOnLaunch: true,
  launchAtLogin: false,
  standaloneWindow: false,
  preserveInput: false,
  showRecent: true,
  appearance: "glass",
  windowEffect: "acrylic",
  glassOpacity: GLASS_OPACITY.def,
  theme: "system",
  accentColor: "violet",
  recentLimit: RECENT_LIMIT.def,
  sidebarWidth: SIDEBAR_WIDTH.def,
  optionsWidth: OPTIONS_WIDTH.def,
  inputHeight: INPUT_HEIGHT.def,
  popoverWidth: 820,
  popoverHeight: 650,
  favorites: [],
  recent: [],
  lastTool: null,
  categoryOrder: CATEGORIES,
  hiddenCategories: [],
  collapsedCategories: [],
  toolOrder: {},
  toolOptions: {},
};
