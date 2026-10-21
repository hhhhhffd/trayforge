import { useState } from "react";
import { Code2, Copy, Download, Eye, FolderOpen } from "lucide-react";
import { api, copyText } from "../api";
import { useToast } from "../toast";

const EXAMPLE_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="200" height="200">
  <defs>
    <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#6366f1"/>
      <stop offset="100%" stop-color="#8b5cf6"/>
    </linearGradient>
  </defs>
  <rect width="200" height="200" rx="20" fill="url(#grad)"/>
  <circle cx="100" cy="75" r="40" fill="white" opacity="0.15"/>
  <circle cx="100" cy="75" r="28" fill="white" opacity="0.9"/>
  <rect x="60" y="128" width="80" height="10" rx="5" fill="white" opacity="0.7"/>
  <rect x="75" y="148" width="50" height="8" rx="4" fill="white" opacity="0.5"/>
  <text x="100" y="180" font-family="system-ui, sans-serif" font-size="11" font-weight="600" fill="white" opacity="0.85" text-anchor="middle">Trayforge</text>
</svg>`;

const SVG_FILTER = [{ name: "SVG", extensions: ["svg"] }];

export default function SvgEditor() {
  const [source, setSource] = useState(EXAMPLE_SVG);
  const [broken, setBroken] = useState(false);
  const toast = useToast();

  // An <img> never executes scripts inside SVG, unlike inline or <object> embedding.
  const previewUrl = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(source)}`;

  const run = async (label: string, action: () => Promise<void>) => {
    try {
      await action();
    } catch (e) {
      toast(`${label} failed: ${String(e)}`);
    }
  };

  const edit = (value: string) => {
    setSource(value);
    setBroken(false);
  };

  const copy = () =>
    run("Copy", async () => {
      await copyText(source);
      toast("SVG copied to clipboard");
    });

  const open = () =>
    run("Open", async () => {
      const path = await api.openFileDialog(SVG_FILTER);
      if (path) edit(await api.readTextFile(path));
    });

  const exportSvg = () =>
    run("Export", async () => {
      const path = await api.saveFileDialog("image.svg", SVG_FILTER);
      if (!path) return;
      await api.writeTextFile(path, source);
      toast("SVG exported");
    });

  return (
    <div className="split2">
      <section className="split-pane">
        <div className="pane-bar">
          <Code2 size={16} aria-hidden="true" />
          <strong>SVG Source</strong>
          <span className="grow" />
          <button className="icon-btn" onClick={copy} title="Copy SVG source" aria-label="Copy SVG source">
            <Copy size={16} />
          </button>
          <button className="icon-btn" onClick={open} title="Open SVG file" aria-label="Open SVG file">
            <FolderOpen size={16} />
          </button>
        </div>
        <textarea
          className="editor flush"
          value={source}
          spellCheck={false}
          aria-label="SVG source"
          onChange={(e) => edit(e.target.value)}
        />
      </section>
      <section className="split-pane">
        <div className="pane-bar">
          <Eye size={16} aria-hidden="true" />
          <strong>Preview</strong>
          <span className="grow" />
          <button className="icon-btn" onClick={exportSvg} title="Export SVG" aria-label="Export SVG">
            <Download size={16} />
          </button>
        </div>
        <div className="preview">
          {broken || !source.trim() ? (
            <div className="empty-state">{source.trim() ? "Invalid SVG markup" : "Nothing to preview"}</div>
          ) : (
            <img src={previewUrl} alt="SVG preview" onError={() => setBroken(true)} />
          )}
        </div>
      </section>
    </div>
  );
}
