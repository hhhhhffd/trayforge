import { useCallback, useEffect, useMemo, useRef, useState, type MutableRefObject } from "react";
import { AlertTriangle, ArrowUpDown, Check, ClipboardPaste, Copy, Play, TextCursorInput, Trash2, XCircle, AlignLeft } from "lucide-react";
import { onAction } from "../actions";
import { api, copyText, toToolFailure } from "../api";
import { countChars } from "../lib";
import { useSettings } from "../settings";
import { useToast } from "../toast";
import { INPUT_HEIGHT, OPTIONS_WIDTH } from "../types";
import type { OptionValue, ToolFailure, ToolInfo, ToolOutput } from "../types";
import { OptionsPanel, resolveOptions } from "./OptionsPanel";
import { Splitter } from "./Splitter";

// Scripts are blocked by `sandbox=""`; the CSP also forbids any network access.
const PREVIEW_HEAD =
  '<meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src \'none\'; img-src data:; style-src \'unsafe-inline\'">' +
  "<style>:root{color-scheme:light dark}body{font:14px system-ui,sans-serif;margin:12px}</style>";

function outputAsText(out: ToolOutput | null): string {
  if (!out) return "";
  if (out.type === "text") return out.value;
  return out.items
    .filter((i) => !i.isImage)
    .map((i) => `${i.label}: ${i.value}`)
    .join("\n");
}

export function StandardTool({ tool, carriedInput }: { tool: ToolInfo; carriedInput: MutableRefObject<string> }) {
  const { settings, update } = useSettings();
  const toast = useToast();
  const [input, setInput] = useState(settings.preserveInput ? carriedInput.current : "");
  const [output, setOutput] = useState<ToolOutput | null>(null);
  const [error, setError] = useState<ToolFailure | null>(null);
  const [reversed, setReversed] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const stdRef = useRef<HTMLDivElement>(null);
  const mainRef = useRef<HTMLDivElement>(null);
  const hasRun = useRef(false);
  const seq = useRef(0);

  const options = useMemo(
    () => resolveOptions(tool.options, settings.toolOptions[tool.id]),
    [tool, settings.toolOptions],
  );
  const canRun = tool.allowsEmptyInput || input.length > 0;

  useEffect(() => {
    carriedInput.current = input;
  }, [input, carriedInput]);

  const run = useCallback(
    async (over: { input?: string; reversed?: boolean; options?: Record<string, OptionValue> } = {}) => {
      const text = over.input ?? input;
      if (!tool.allowsEmptyInput && text.length === 0) return;
      const id = ++seq.current;
      hasRun.current = true;
      try {
        const out = await api.processTool(tool.id, text, over.options ?? options, over.reversed ?? reversed);
        if (id !== seq.current) return;
        setOutput(out);
        setError(null);
      } catch (e) {
        // The previous successful output stays visible next to the error.
        if (id === seq.current) setError(toToolFailure(e));
      }
    },
    [tool, input, options, reversed],
  );

  const copyOutput = useCallback(
    async (text: string, key: string) => {
      if (!text) return;
      try {
        await copyText(text);
        setCopied(key);
        window.setTimeout(() => setCopied((c) => (c === key ? null : c)), 1500);
      } catch (e) {
        toast(`Copy failed: ${String(e)}`);
      }
    },
    [toast],
  );

  const clearAll = useCallback(() => {
    seq.current++;
    hasRun.current = false;
    setInput("");
    setOutput(null);
    setError(null);
  }, []);

  const swap = useCallback(() => {
    if (!tool.secondaryAction) return;
    seq.current++;
    if (output?.type === "text") setInput(output.value);
    setOutput(null);
    setError(null);
    setReversed((r) => !r);
  }, [tool.secondaryAction, output]);

  // Shortcuts forwarded from App; the handler is re-subscribed when its inputs change.
  useEffect(
    () =>
      onAction((a) => {
        if (a === "process") void run();
        else if (a === "copy") void copyOutput(outputAsText(output), "out");
        else if (a === "clear") clearAll();
        else if (a === "swap") swap();
      }),
    [run, copyOutput, clearAll, swap, output],
  );

  const setOption = (id: string, value: OptionValue) => {
    const next = { ...options, [id]: value };
    update({ toolOptions: { ...settings.toolOptions, [tool.id]: next } });
    // Auto-process only once the user has run the tool (`hasRun`).
    if (hasRun.current && canRun) void run({ options: next });
  };

  const setMode = (rev: boolean) => {
    setReversed(rev);
    if (hasRun.current && canRun) void run({ reversed: rev });
  };

  const paste = async () => {
    try {
      setInput(await navigator.clipboard.readText());
    } catch (e) {
      toast(`Paste failed: ${String(e)}`);
    }
  };

  const onDrop = async (e: React.DragEvent) => {
    const file = e.dataTransfer.files[0];
    if (!tool.supportsFileInput || !file) return;
    e.preventDefault();
    setInput(await file.text());
  };

  const actionLabel = reversed && tool.secondaryAction ? tool.secondaryAction : tool.primaryAction;
  const text = output?.type === "text" ? output.value : "";

  return (
    <div className="std" ref={stdRef}>
      <div className="std-main scroll" ref={mainRef}>
        {tool.secondaryAction && (
          <div className="modes" role="radiogroup" aria-label="Mode">
            {[tool.primaryAction, tool.secondaryAction].map((label, i) => (
              <button
                key={label}
                role="radio"
                aria-checked={reversed === (i === 1)}
                className={reversed === (i === 1) ? "on" : ""}
                onClick={() => setMode(i === 1)}
              >
                {label}
              </button>
            ))}
          </div>
        )}

        <section className="pane">
          <div className="pane-head">
            <TextCursorInput size={16} aria-hidden="true" />
            <label htmlFor="tool-input">Input</label>
            <span className="grow" />
            <span className="muted">{countChars(input)}</span>
            <button className="icon-btn" onClick={paste} title="Paste from clipboard" aria-label="Paste">
              <ClipboardPaste size={16} />
            </button>
            <button
              className="icon-btn"
              onClick={() => setInput("")}
              title="Clear input"
              aria-label="Clear input"
              disabled={!input}
            >
              <XCircle size={16} />
            </button>
          </div>
          <textarea
            id="tool-input"
            className="editor in"
            style={{ height: settings.inputHeight }}
            value={input}
            placeholder={tool.inputPlaceholder}
            spellCheck={false}
            autoFocus
            onChange={(e) => setInput(e.target.value)}
            onDragOver={(e) => tool.supportsFileInput && e.preventDefault()}
            onDrop={onDrop}
          />
        </section>

        <Splitter
          orientation="horizontal"
          label="Resize input"
          value={settings.inputHeight}
          min={INPUT_HEIGHT.min}
          max={() => (mainRef.current ? mainRef.current.clientHeight - 200 : INPUT_HEIGHT.max)}
          defaultValue={INPUT_HEIGHT.def}
          onChange={(inputHeight) => update({ inputHeight })}
        />

        <div className="actions">
          <button className="primary" disabled={!canRun} onClick={() => void run()} title="Run (Ctrl+Enter)">
            <Play size={14} fill="currentColor" aria-hidden="true" /> {actionLabel}
          </button>
          <span className="grow" />
          {tool.secondaryAction && (
            <button className="soft" onClick={swap} title="Swap input and output (Ctrl+Shift+S)">
              <ArrowUpDown size={15} aria-hidden="true" /> Swap
            </button>
          )}
          <button className="soft" onClick={clearAll} title="Clear input and output (Ctrl+L)">
            <Trash2 size={15} aria-hidden="true" /> Clear All
          </button>
        </div>

        <section className="pane grow-pane">
          <div className="pane-head">
            <AlignLeft size={16} aria-hidden="true" />
            <span className="pane-title">Output</span>
            <span className="grow" />
            {output?.type === "text" && <span className="muted">{countChars(text)}</span>}
            <button
              className="soft small"
              disabled={!outputAsText(output)}
              onClick={() => void copyOutput(outputAsText(output), "out")}
              title="Copy output (Ctrl+Shift+C)"
            >
              {copied === "out" ? <Check size={14} /> : <Copy size={14} />}
              {copied === "out" ? "Copied" : "Copy"}
            </button>
          </div>
          {output?.type === "multiple" ? (
            <div className="items">
              {output.items.map((item, i) => (
                <div key={i} className={`item${item.isWarning ? " warn" : ""}`}>
                  <div className="item-head">
                    {item.isWarning && <AlertTriangle size={14} aria-label="Warning" />}
                    <span>{item.label}</span>
                    <span className="grow" />
                    {!item.isImage && (
                      <button
                        className="soft small"
                        onClick={() => void copyOutput(item.value, `item-${i}`)}
                        aria-label={`Copy ${item.label}`}
                      >
                        {copied === `item-${i}` ? <Check size={14} /> : <Copy size={14} />}
                        {copied === `item-${i}` ? "Copied" : "Copy"}
                      </button>
                    )}
                  </div>
                  {item.isImage ? (
                    <img src={item.value} alt={item.label} />
                  ) : (
                    <pre className="item-value">{item.value}</pre>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <div className={`editor out${text ? "" : " placeholder"}`} aria-live="polite">
              {text || tool.outputPlaceholder}
            </div>
          )}
          {tool.id === "markdown-preview" && text && (
            <iframe className="md-preview" title="Markdown preview" sandbox="" srcDoc={PREVIEW_HEAD + text} />
          )}
          {error && (
            <div className="error-box" role="alert">
              <AlertTriangle size={16} aria-hidden="true" />
              <div>
                <strong>Error</strong>
                <p>{error.display}</p>
                <small>Check your input and try again</small>
              </div>
            </div>
          )}
        </section>
      </div>
      {tool.options.length > 0 && (
        <>
          <Splitter
            orientation="vertical"
            label="Resize options"
            invert
            value={settings.optionsWidth}
            min={OPTIONS_WIDTH.min}
            max={() => (stdRef.current ? stdRef.current.clientWidth / 2 : 4000)}
            defaultValue={OPTIONS_WIDTH.def}
            onChange={(optionsWidth) => update({ optionsWidth })}
          />
          <OptionsPanel options={tool.options} values={options} width={settings.optionsWidth} onChange={setOption} />
        </>
      )}
    </div>
  );
}
