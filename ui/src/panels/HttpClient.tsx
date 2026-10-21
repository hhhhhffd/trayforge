import { useRef, useState } from "react";
import { Copy, Globe, Plus, Send, X } from "lucide-react";
import { api, copyText, toToolFailure } from "../api";
import { useToast } from "../toast";
import type { HttpResponse } from "../types";

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];
type BodyFormat = "None" | "JSON" | "Form" | "Text";
const BODY_FORMATS: BodyFormat[] = ["None", "JSON", "Form", "Text"];

interface Row {
  id: number;
  key: string;
  value: string;
  enabled: boolean;
}

let nextRowId = 1;
const newRow = (key = "", value = ""): Row => ({ id: nextRowId++, key, value, enabled: true });

function RowsEditor({
  rows,
  onChange,
  addLabel,
}: {
  rows: Row[];
  onChange: (rows: Row[]) => void;
  addLabel: string;
}) {
  const patch = (id: number, p: Partial<Row>) => onChange(rows.map((r) => (r.id === id ? { ...r, ...p } : r)));
  return (
    <div className="kv">
      {rows.map((r) => (
        <div className="kv-row" key={r.id}>
          <input type="checkbox" checked={r.enabled} aria-label="Enabled" onChange={(e) => patch(r.id, { enabled: e.target.checked })} />
          <input className="field mono" value={r.key} placeholder="Key" spellCheck={false} aria-label="Key" onChange={(e) => patch(r.id, { key: e.target.value })} />
          <input className="field mono" value={r.value} placeholder="Value" spellCheck={false} aria-label="Value" onChange={(e) => patch(r.id, { value: e.target.value })} />
          <button className="icon-btn" aria-label="Remove row" title="Remove" onClick={() => onChange(rows.filter((x) => x.id !== r.id))}>
            <X size={15} />
          </button>
        </div>
      ))}
      <button className="soft small" onClick={() => onChange([...rows, newRow()])}>
        <Plus size={14} /> {addLabel}
      </button>
    </div>
  );
}

const activeRows = (rows: Row[]) => rows.filter((r) => r.enabled && r.key.trim());

function prettyBody(res: HttpResponse): string {
  try {
    const parsed = JSON.parse(res.body);
    if (parsed && typeof parsed === "object") return JSON.stringify(parsed, null, 2);
  } catch {
    // Not JSON: show the raw body.
  }
  return res.body;
}

export default function HttpClient() {
  const toast = useToast();
  const [method, setMethod] = useState("GET");
  const [url, setUrl] = useState("https://httpbin.org/get");
  const [reqTab, setReqTab] = useState<"Headers" | "Body">("Body");
  const [headers, setHeaders] = useState<Row[]>(() => [newRow("Accept", "*/*"), newRow("User-Agent", "Trayforge")]);
  const [format, setFormat] = useState<BodyFormat>("None");
  const [text, setText] = useState("");
  const [form, setForm] = useState<Row[]>(() => [newRow()]);
  const [resTab, setResTab] = useState<"Body" | "Headers" | "Info">("Body");
  const [busy, setBusy] = useState(false);
  const [response, setResponse] = useState<{ res: HttpResponse; url: string; method: string } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [split, setSplit] = useState(55);
  const root = useRef<HTMLDivElement>(null);

  const send = async () => {
    let target = url.trim();
    if (!target) return;
    if (!/^[a-z][a-z0-9+.-]*:\/\//i.test(target)) target = `https://${target}`;
    const hdrs: [string, string][] = activeRows(headers).map((r) => [r.key.trim(), r.value]);
    const hasHeader = (name: string) => hdrs.some(([k]) => k.toLowerCase() === name);
    let body: string | null = null;
    if (format === "JSON" || format === "Text") {
      body = text;
      if (!hasHeader("content-type")) hdrs.push(["Content-Type", format === "JSON" ? "application/json" : "text/plain"]);
    } else if (format === "Form") {
      body = new URLSearchParams(activeRows(form).map((r) => [r.key.trim(), r.value])).toString();
      if (!hasHeader("content-type")) hdrs.push(["Content-Type", "application/x-www-form-urlencoded"]);
    }
    setBusy(true);
    setError(null);
    try {
      const res = await api.httpRequest({ method, url: target, headers: hdrs, body });
      setResponse({ res, url: target, method });
    } catch (e) {
      setResponse(null);
      setError(toToolFailure(e).display);
    } finally {
      setBusy(false);
    }
  };

  const startDrag = (e: React.PointerEvent) => {
    e.currentTarget.setPointerCapture(e.pointerId);
  };
  const onDrag = (e: React.PointerEvent) => {
    if (!e.currentTarget.hasPointerCapture(e.pointerId) || !root.current) return;
    const rect = root.current.getBoundingClientRect();
    setSplit(Math.min(80, Math.max(25, ((e.clientX - rect.left) / rect.width) * 100)));
  };

  const copyBody = async () => {
    if (!response) return;
    try {
      await copyText(prettyBody(response.res));
      toast("Response body copied");
    } catch (e) {
      toast(`Copy failed: ${String(e)}`);
    }
  };

  const res = response?.res;
  const statusClass = res ? (res.status < 300 ? "ok" : res.status < 400 ? "redir" : "bad") : "";
  const contentType = res?.headers.find(([k]) => k.toLowerCase() === "content-type")?.[1] ?? "—";

  return (
    <div className="http" ref={root}>
      <section className="http-req" style={{ width: `${split}%` }}>
        <div className="http-bar">
          <select value={method} onChange={(e) => setMethod(e.target.value)} aria-label="Method">
            {METHODS.map((m) => (
              <option key={m}>{m}</option>
            ))}
          </select>
          <input
            className="field mono grow"
            value={url}
            spellCheck={false}
            aria-label="URL"
            placeholder="https://api.example.com/endpoint"
            onChange={(e) => setUrl(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void send()}
          />
          <button className="primary" onClick={() => void send()} disabled={busy || !url.trim()}>
            <Send size={14} aria-hidden="true" /> {busy ? "Sending…" : "Send"}
          </button>
        </div>
        <div className="tabs" role="tablist">
          {(["Headers", "Body"] as const).map((t) => (
            <button key={t} role="tab" aria-selected={reqTab === t} className={reqTab === t ? "on" : ""} onClick={() => setReqTab(t)}>
              {t}
              {t === "Headers" && <span className="count">{activeRows(headers).length}</span>}
            </button>
          ))}
        </div>
        <div className="http-content scroll">
          {reqTab === "Headers" ? (
            <RowsEditor rows={headers} onChange={setHeaders} addLabel="Add Header" />
          ) : (
            <>
              <div className="format">
                <span className="muted">Format:</span>
                <div className="seg" role="radiogroup" aria-label="Body format">
                  {BODY_FORMATS.map((f) => (
                    <button key={f} role="radio" aria-checked={format === f} className={format === f ? "on" : ""} onClick={() => setFormat(f)}>
                      {f}
                    </button>
                  ))}
                </div>
              </div>
              {format === "None" && <div className="empty-state">No body</div>}
              {(format === "JSON" || format === "Text") && (
                <textarea
                  className="editor flush grow"
                  value={text}
                  spellCheck={false}
                  aria-label="Request body"
                  placeholder={format === "JSON" ? '{ "key": "value" }' : "Request body"}
                  onChange={(e) => setText(e.target.value)}
                />
              )}
              {format === "Form" && <RowsEditor rows={form} onChange={setForm} addLabel="Add Field" />}
            </>
          )}
        </div>
      </section>

      <div
        className="divider"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize panels"
        tabIndex={0}
        onPointerDown={startDrag}
        onPointerMove={onDrag}
        onKeyDown={(e) => {
          if (e.key === "ArrowLeft") setSplit((s) => Math.max(25, s - 3));
          if (e.key === "ArrowRight") setSplit((s) => Math.min(80, s + 3));
        }}
      />

      <section className="http-res">
        <div className="res-status">
          <span className={`dot ${statusClass}`} />
          {res ? (
            <>
              <strong>
                {res.status} {res.statusText}
              </strong>
              <span className="muted">
                {res.durationMs} ms · {formatBytes(res.sizeBytes)}
              </span>
            </>
          ) : (
            <span className="muted">{error ? "Request failed" : "No Request/Response"}</span>
          )}
          <span className="grow" />
          {res && resTab === "Body" && (
            <button className="soft small" onClick={() => void copyBody()}>
              <Copy size={14} /> Copy
            </button>
          )}
        </div>
        <div className="tabs" role="tablist">
          {(["Body", "Headers", "Info"] as const).map((t) => (
            <button key={t} role="tab" aria-selected={resTab === t} className={resTab === t ? "on" : ""} onClick={() => setResTab(t)}>
              {t}
            </button>
          ))}
        </div>
        <div className="http-content scroll">
          {error && (
            <div className="error-box" role="alert">
              <div>
                <strong>Error</strong>
                <p>{error}</p>
              </div>
            </div>
          )}
          {!res && !error && (
            <div className="empty-state">
              <Globe size={40} aria-hidden="true" />
              No Request/Response
            </div>
          )}
          {res && resTab === "Body" && <pre className="resp-body">{prettyBody(res) || "(empty body)"}</pre>}
          {res && resTab === "Headers" && (
            <table className="info-table">
              <tbody>
                {res.headers.map(([k, v], i) => (
                  <tr key={i}>
                    <th>{k}</th>
                    <td>{v}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
          {res && response && resTab === "Info" && (
            <table className="info-table">
              <tbody>
                {(
                  [
                    ["Status", `${res.status} ${res.statusText}`],
                    ["Method", response.method],
                    ["URL", response.url],
                    ["Duration", `${res.durationMs} ms`],
                    ["Size", `${formatBytes(res.sizeBytes)} (${res.sizeBytes} bytes)`],
                    ["Content-Type", contentType],
                    ["Headers", String(res.headers.length)],
                  ] as const
                ).map(([k, v]) => (
                  <tr key={k}>
                    <th>{k}</th>
                    <td>{v}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      </section>
    </div>
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}
