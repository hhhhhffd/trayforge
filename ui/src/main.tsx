import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { api } from "./api";
import { normalizeSettings, SettingsProvider } from "./settings";
import { ToastProvider } from "./toast";
import type { Settings, ToolInfo } from "./types";
import "./styles.css";

// Browser-only development has no Acrylic backdrop behind the window.
if (!("__TAURI_INTERNALS__" in window)) document.documentElement.dataset.dev = "";

interface Loaded {
  tools: ToolInfo[];
  raw: Record<string, unknown>;
  settings: Settings;
}

function Root() {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.listTools(), api.getSettings()])
      .then(([tools, raw]) => setLoaded({ tools, raw, settings: normalizeSettings(raw) }))
      .catch((e) => setError(String(e instanceof Error ? e.message : (e as { display?: string }).display ?? e)));
  }, []);

  if (error) {
    return (
      <div className="app-error" role="alert">
        <strong>Trayforge failed to start</strong>
        <p>{error}</p>
      </div>
    );
  }
  if (!loaded) return null;

  return (
    <SettingsProvider initial={loaded.settings} rawExtras={loaded.raw}>
      <ToastProvider>
        <App tools={loaded.tools} />
      </ToastProvider>
    </SettingsProvider>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
);
