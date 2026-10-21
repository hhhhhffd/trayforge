import { useEffect, useRef, useState } from "react";
import { Eye, EyeOff, FolderOpen, ClipboardPaste, QrCode, UserRound } from "lucide-react";
import { api, copyText, toToolFailure } from "../api";
import { useToast } from "../toast";
import type { TotpAlgorithm, TotpCode } from "../types";

const ALGORITHMS: TotpAlgorithm[] = ["SHA1", "SHA256", "SHA512"];
const RING_RADIUS = 20;
const RING_LENGTH = 2 * Math.PI * RING_RADIUS;

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).split(",")[1] ?? "");
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

export default function TotpGenerator() {
  const toast = useToast();
  const [account, setAccount] = useState("");
  const [issuer, setIssuer] = useState("");
  const [secret, setSecret] = useState("");
  const [reveal, setReveal] = useState(false);
  const [algorithm, setAlgorithm] = useState<TotpAlgorithm>("SHA1");
  const [digits, setDigits] = useState(6);
  const [period, setPeriod] = useState(30);
  const [code, setCode] = useState<TotpCode | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [qrError, setQrError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);

  // The only recurring timer in the app: ticks once per second while the panel is
  // mounted and the document is visible; stops when either stops being true.
  useEffect(() => {
    const cleanSecret = secret.replace(/[\s-]/g, "");
    if (!cleanSecret) {
      setCode(null);
      setError(null);
      return;
    }
    let cancelled = false;
    const tick = async () => {
      try {
        const next = await api.totpGenerate({ secret: cleanSecret, algorithm, digits, period });
        if (cancelled) return;
        setCode(next);
        setError(null);
      } catch (e) {
        if (cancelled) return;
        setCode(null);
        setError(toToolFailure(e).display);
      }
    };
    let timer: number | undefined;
    const stop = () => {
      window.clearInterval(timer);
      timer = undefined;
    };
    const start = () => {
      if (timer !== undefined) return;
      void tick();
      timer = window.setInterval(() => void tick(), 1000);
    };
    const onVisibility = () => (document.hidden ? stop() : start());
    if (!document.hidden) start();
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      cancelled = true;
      stop();
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [secret, algorithm, digits, period]);

  const importImage = async (blob: Blob) => {
    setQrError(null);
    try {
      const text = await api.decodeQrImage(await blobToBase64(blob));
      if (!text.toLowerCase().startsWith("otpauth://")) {
        throw new Error("The QR code does not contain an otpauth:// URI");
      }
      const parsed = await api.totpParseUri(text);
      setAccount(parsed.account);
      setIssuer(parsed.issuer);
      setSecret(parsed.secret);
      setAlgorithm(parsed.algorithm);
      setDigits(parsed.digits);
      setPeriod(parsed.period);
    } catch (e) {
      setQrError(e instanceof Error ? e.message : toToolFailure(e).display);
    }
  };

  const pasteImage = async () => {
    try {
      for (const item of await navigator.clipboard.read()) {
        const type = item.types.find((t) => t.startsWith("image/"));
        if (type) return void (await importImage(await item.getType(type)));
      }
      setQrError("The clipboard does not contain an image");
    } catch (e) {
      setQrError(`Could not read the clipboard: ${String(e)}`);
    }
  };

  const copyCode = async () => {
    if (!code) return;
    try {
      await copyText(code.code);
      toast("Code copied to clipboard");
    } catch (e) {
      toast(`Copy failed: ${String(e)}`);
    }
  };

  const setPeriodClamped = (n: number) => {
    if (Number.isFinite(n)) setPeriod(Math.min(300, Math.max(1, Math.trunc(n))));
  };

  const grouped = code ? code.code.replace(/(.{3})(?=.)/g, "$1 ") : "";

  return (
    <div className="totp scroll">
      <h3 className="section-title">
        <QrCode size={16} aria-hidden="true" /> IMPORT FROM QR CODE
      </h3>
      <div
        className={`drop${dragging ? " over" : ""}`}
        onDragOver={(e) => {
          e.preventDefault();
          setDragging(true);
        }}
        onDragLeave={() => setDragging(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDragging(false);
          const file = e.dataTransfer.files[0];
          if (file) void importImage(file);
        }}
      >
        <QrCode size={36} aria-hidden="true" />
        <p>Drop a QR code screenshot here</p>
        <div className="drop-actions">
          <button className="soft" onClick={() => fileInput.current?.click()}>
            <FolderOpen size={15} aria-hidden="true" /> Choose Image...
          </button>
          <button className="soft" onClick={() => void pasteImage()}>
            <ClipboardPaste size={15} aria-hidden="true" /> Paste Image
          </button>
        </div>
        <input
          ref={fileInput}
          type="file"
          accept="image/*"
          hidden
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void importImage(file);
            e.target.value = "";
          }}
        />
      </div>
      {qrError && (
        <div className="error-box" role="alert">
          <div>
            <strong>Error</strong>
            <p>{qrError}</p>
          </div>
        </div>
      )}

      <h3 className="section-title">
        <UserRound size={16} aria-hidden="true" /> ACCOUNT DETAILS
      </h3>
      <div className="form-card">
        <label className="form-row">
          <span>Account</span>
          <input className="field" value={account} placeholder="name@example.com" spellCheck={false} onChange={(e) => setAccount(e.target.value)} />
        </label>
        <label className="form-row">
          <span>Issuer</span>
          <input className="field" value={issuer} placeholder="e.g. GitHub (optional)" spellCheck={false} onChange={(e) => setIssuer(e.target.value)} />
        </label>
        <label className="form-row">
          <span>Secret</span>
          <input
            className="field mono"
            type={reveal ? "text" : "password"}
            value={secret}
            placeholder="Base32 secret"
            spellCheck={false}
            autoComplete="off"
            onChange={(e) => setSecret(e.target.value)}
          />
          <button
            type="button"
            className="icon-btn"
            onClick={() => setReveal((r) => !r)}
            aria-label={reveal ? "Hide secret" : "Show secret"}
            title={reveal ? "Hide secret" : "Show secret"}
          >
            {reveal ? <EyeOff size={17} /> : <Eye size={17} />}
          </button>
        </label>
        <div className="form-row">
          <span>Algorithm</span>
          <div className="seg" role="radiogroup" aria-label="Algorithm">
            {ALGORITHMS.map((a) => (
              <button key={a} role="radio" aria-checked={algorithm === a} className={algorithm === a ? "on" : ""} onClick={() => setAlgorithm(a)}>
                {a}
              </button>
            ))}
          </div>
        </div>
        <div className="form-row">
          <span>Digits</span>
          <div className="seg" role="radiogroup" aria-label="Digits">
            {[6, 8].map((d) => (
              <button key={d} role="radio" aria-checked={digits === d} className={digits === d ? "on" : ""} onClick={() => setDigits(d)}>
                {d}
              </button>
            ))}
          </div>
        </div>
        <div className="form-row">
          <span>Period</span>
          <div className="stepper">
            <button aria-label="Decrease period" disabled={period <= 1} onClick={() => setPeriodClamped(period - 1)}>
              −
            </button>
            <input type="number" min={1} max={300} value={period} aria-label="Period in seconds" onChange={(e) => setPeriodClamped(e.target.valueAsNumber)} />
            <button aria-label="Increase period" disabled={period >= 300} onClick={() => setPeriodClamped(period + 1)}>
              +
            </button>
          </div>
          <span className="muted">seconds</span>
        </div>
      </div>

      {error && (
        <div className="error-box" role="alert">
          <div>
            <strong>Error</strong>
            <p>{error}</p>
          </div>
        </div>
      )}
      {code && (
        <button className="code-card" onClick={() => void copyCode()} title="Copy code">
          <span className="code">{grouped}</span>
          <span className="ring" aria-label={`${code.remainingSeconds} seconds remaining`}>
            <svg viewBox="0 0 48 48" width="52" height="52" aria-hidden="true">
              <circle cx="24" cy="24" r={RING_RADIUS} className="ring-bg" />
              <circle
                cx="24"
                cy="24"
                r={RING_RADIUS}
                className={`ring-fg${code.remainingSeconds <= 5 ? " low" : ""}`}
                strokeDasharray={RING_LENGTH}
                strokeDashoffset={RING_LENGTH * (1 - code.remainingSeconds / period)}
                transform="rotate(-90 24 24)"
              />
            </svg>
            <span className="ring-text">{code.remainingSeconds}</span>
          </span>
        </button>
      )}
    </div>
  );
}
