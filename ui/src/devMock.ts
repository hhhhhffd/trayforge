// Browser-only fake of the Tauri commands. Loaded lazily by api.ts when
// `window.__TAURI_INTERNALS__` is absent; never part of the Tauri code path.
import type { ToolInfo } from "./types";

const SETTINGS_KEY = "trayforge-dev-settings";
const TOTP_ACCOUNTS_KEY = "trayforge-dev-totp-accounts";
const DRAFTS_KEY = "trayforge-dev-drafts";

// Accounts persist like the encrypted file; drafts last for the tab like the tray process.
function readJson(storage: Storage, key: string, fallback: unknown): unknown {
  return JSON.parse(storage.getItem(key) ?? JSON.stringify(fallback));
}

const extraTools: ToolInfo[] = [
  ["http-client", "HTTP Client", "globe", "Build and send HTTP requests with full header and body control"],
  ["totp-generator", "TOTP Generator", "shield-check", "Generate live TOTP codes from a secret or QR code, for testing 2FA integrations"],
].map(([id, name, icon, description]) => ({
  id,
  name,
  category: "Dev Utils" as const,
  description,
  keywords: [name.toLowerCase()],
  icon,
  inputPlaceholder: "",
  outputPlaceholder: "",
  primaryAction: "Run",
  secondaryAction: null,
  supportsFileInput: false,
  allowsEmptyInput: true,
  customUi: true,
  options: [],
}));

const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function mockInvoke(cmd: string, args: Record<string, unknown> = {}): Promise<unknown> {
  switch (cmd) {
    case "list_tools": {
      const tools: ToolInfo[] = await fetch("/dev-tools.json").then((r) => r.json());
      const ids = new Set(tools.map((t) => t.id));
      return [...tools, ...extraTools.filter((t) => !ids.has(t.id))];
    }
    case "process_tool": {
      const input = String(args.input ?? "");
      if (input.startsWith("!")) {
        throw { kind: "invalidInput", message: "Mock failure", display: "Invalid input: Mock failure" };
      }
      if (input.startsWith("multi")) {
        const svg =
          "data:image/svg+xml;utf8," +
          encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" rx="12" fill="#8b5cf6"/></svg>');
        return {
          type: "multiple",
          items: [
            { label: "Upper", value: input.toUpperCase(), isWarning: false, isImage: false },
            { label: "Reversed", value: [...input].reverse().join(""), isWarning: false, isImage: false },
            { label: "Heads up", value: "This is a warning item", isWarning: true, isImage: false },
            { label: "Image", value: svg, isWarning: false, isImage: true },
          ],
        };
      }
      const value = args.reverse ? input.toLowerCase() : input.toUpperCase();
      return { type: "text", value, metadata: {} };
    }
    case "get_settings":
      return JSON.parse(localStorage.getItem(SETTINGS_KEY) ?? "{}");
    case "save_settings":
      localStorage.setItem(SETTINGS_KEY, JSON.stringify(args.settings));
      return null;
    case "http_request": {
      await wait(250);
      const req = args.request as { url: string; method: string };
      const body = JSON.stringify({ url: req.url, method: req.method, origin: "127.0.0.1" }, null, 2);
      return {
        status: 200,
        statusText: "OK",
        headers: [["content-type", "application/json"], ["server", "mock"]],
        body,
        durationMs: 250,
        sizeBytes: body.length,
      };
    }
    case "totp_generate": {
      const period = Number(args.period);
      const digits = Number(args.digits);
      const now = Math.floor(Date.now() / 1000);
      if (!String(args.secret).trim()) throw "Secret is empty";
      return {
        code: String(Math.floor(now / period) % 10 ** digits).padStart(digits, "0"),
        remainingSeconds: period - (now % period),
      };
    }
    case "totp_parse_uri":
      return { account: "alice@example.com", issuer: "Example", secret: "JBSWY3DPEHPK3PXP", algorithm: "SHA1", digits: 6, period: 30 };
    case "totp_load_accounts":
      return readJson(localStorage, TOTP_ACCOUNTS_KEY, []);
    case "totp_save_accounts":
      localStorage.setItem(TOTP_ACCOUNTS_KEY, JSON.stringify(args.accounts));
      return null;
    case "get_drafts":
      return readJson(sessionStorage, DRAFTS_KEY, {});
    case "set_draft": {
      const drafts = readJson(sessionStorage, DRAFTS_KEY, {}) as Record<string, unknown>;
      if (args.value === null) delete drafts[String(args.key)];
      else drafts[String(args.key)] = args.value;
      sessionStorage.setItem(DRAFTS_KEY, JSON.stringify(drafts));
      return null;
    }
    case "decode_qr_image":
      return "otpauth://totp/Example:alice@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Example";
    case "pick_screen_color":
      return "#" + Math.floor(Math.random() * 0xffffff).toString(16).padStart(6, "0").toUpperCase();
    case "read_text_file":
      return '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" fill="#10b981"/></svg>';
    case "write_text_file":
    case "hide_window":
    case "set_launch_at_login":
    case "reset_window_state":
      return null;
    default:
      throw `Unknown mock command: ${cmd}`;
  }
}
