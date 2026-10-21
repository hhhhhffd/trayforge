import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  HttpRequest,
  HttpResponse,
  OptionValue,
  ToolFailure,
  ToolInfo,
  ToolOutput,
  TotpAlgorithm,
  TotpCode,
  TotpUri,
} from "./types";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/** Real IPC inside Tauri; the dev mock (browser-only development) otherwise. */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return invoke<T>(cmd, args);
  const { mockInvoke } = await import("./devMock");
  return mockInvoke(cmd, args) as Promise<T>;
}

export function toToolFailure(e: unknown): ToolFailure {
  if (typeof e === "string") return { kind: "unknown", display: e };
  if (e && typeof e === "object") {
    const o = e as Partial<ToolFailure>;
    if (typeof o.display === "string") return o as ToolFailure;
    if (typeof o.message === "string") return { kind: o.kind ?? "unknown", display: o.message };
  }
  return { kind: "unknown", display: String(e) };
}

export const api = {
  /** Route requested by the shell (tray "Settings"); resolves to an unlisten function. */
  onNavigate: (handler: (route: string) => void) =>
    inTauri ? listen<string>("navigate", (e) => handler(e.payload)) : Promise.resolve(() => {}),
  /** The shell is about to destroy the window after a short fade-out. */
  onClosing: (handler: () => void) =>
    inTauri ? listen("closing", handler) : Promise.resolve(() => {}),
  /** Moves the frameless window with the mouse; a no-op in the browser. */
  startDragging: () => (inTauri ? getCurrentWindow().startDragging() : Promise.resolve()),
  resetWindowState: () => call<null>("reset_window_state"),
  /** Opens a link in the default browser; the browser mock just uses a new tab. */
  openExternal: (url: string) => (inTauri ? openUrl(url) : Promise.resolve(void window.open(url, "_blank"))),
  listTools: () => call<ToolInfo[]>("list_tools"),
  processTool: (
    toolId: string,
    input: string,
    options: Record<string, OptionValue>,
    reverse: boolean,
  ) => call<ToolOutput>("process_tool", { toolId, input, options, reverse }),
  getSettings: () => call<Record<string, unknown>>("get_settings"),
  saveSettings: (settings: unknown) => call<null>("save_settings", { settings }),
  httpRequest: (request: HttpRequest) => call<HttpResponse>("http_request", { request }),
  totpGenerate: (p: { secret: string; algorithm: TotpAlgorithm; digits: number; period: number }) =>
    call<TotpCode>("totp_generate", p),
  totpParseUri: (uri: string) => call<TotpUri>("totp_parse_uri", { uri }),
  decodeQrImage: (imageBase64: string) => call<string>("decode_qr_image", { imageBase64 }),
  pickScreenColor: () => call<string>("pick_screen_color"),
  readTextFile: (path: string) => call<string>("read_text_file", { path }),
  writeTextFile: (path: string, contents: string) =>
    call<null>("write_text_file", { path, contents }),
  hideWindow: () => call<null>("hide_window"),
  setLaunchAtLogin: (enabled: boolean) => call<null>("set_launch_at_login", { enabled }),

  /** Native open dialog; returns the chosen path or null. Mocked in the browser. */
  async openFileDialog(filters: { name: string; extensions: string[] }[]): Promise<string | null> {
    if (!inTauri) return "C:\\mock\\image.svg";
    const { open } = await import("@tauri-apps/plugin-dialog");
    const r = await open({ multiple: false, directory: false, filters });
    return typeof r === "string" ? r : null;
  },
  async saveFileDialog(
    defaultPath: string,
    filters: { name: string; extensions: string[] }[],
  ): Promise<string | null> {
    if (!inTauri) return "C:\\mock\\export.svg";
    const { save } = await import("@tauri-apps/plugin-dialog");
    return save({ defaultPath, filters });
  },
};

export async function copyText(text: string): Promise<void> {
  await navigator.clipboard.writeText(text);
}
