import { useEffect, useRef, useState } from "react";
import { Eye, EyeOff, FolderOpen, ClipboardPaste, KeyRound, Plus, QrCode, Trash2, UserRound } from "lucide-react";
import { api, copyText, toToolFailure } from "../api";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { useDraft } from "../drafts";
import { useToast } from "../toast";
import type { TotpAlgorithm, TotpCode, TotpUri } from "../types";

const ALGORITHMS: TotpAlgorithm[] = ["SHA1", "SHA256", "SHA512"];
const RING_RADIUS = 20;
const RING_LENGTH = 2 * Math.PI * RING_RADIUS;

type CodeResult = { code: TotpCode; error: null } | { code: null; error: string };

function blobToBase64(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result).split(",")[1] ?? "");
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

const cleanSecret = (secret: string) => secret.replace(/[\s-]/g, "");

const groupCode = (code: string) => code.replace(/(.{3})(?=.)/g, "$1 ");

/** Live codes for `entries`, in the same order. */
function useTotpCodes(entries: Pick<TotpUri, "secret" | "algorithm" | "digits" | "period">[]): (CodeResult | undefined)[] {
  // Entries are compared by value, so re-renders do not restart the timer.
  const key = JSON.stringify(entries.map((e) => [e.secret, e.algorithm, e.digits, e.period]));
  const [state, setState] = useState<{ key: string; results: CodeResult[] }>({ key: "[]", results: [] });

  // The app's only recurring timers: tick once per second while the panel is mounted and
  // the document is visible; stop when either stops being true.
  useEffect(() => {
    const list = JSON.parse(key) as [string, TotpAlgorithm, number, number][];
    if (list.length === 0) return;
    let cancelled = false;
    const tick = async () => {
      const results = await Promise.all(
        list.map(async ([secret, algorithm, digits, period]): Promise<CodeResult> => {
          try {
            return { code: await api.totpGenerate({ secret, algorithm, digits, period }), error: null };
          } catch (e) {
            return { code: null, error: toToolFailure(e).display };
          }
        }),
      );
      if (!cancelled) setState({ key, results });
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
  }, [key]);

  // Results of the previous entries must not be shown next to the new ones.
  return state.key === key ? state.results : [];
}

function Ring({ remaining, period, size }: { remaining: number; period: number; size: number }) {
  return (
    <span className="ring" aria-label={`${remaining} seconds remaining`}>
      <svg viewBox="0 0 48 48" width={size} height={size} aria-hidden="true">
        <circle cx="24" cy="24" r={RING_RADIUS} className="ring-bg" />
        <circle
          cx="24"
          cy="24"
          r={RING_RADIUS}
          className={`ring-fg${remaining <= 5 ? " low" : ""}`}
          strokeDasharray={RING_LENGTH}
          strokeDashoffset={RING_LENGTH * (1 - remaining / period)}
          transform="rotate(-90 24 24)"
        />
      </svg>
      <span className="ring-text">{remaining}</span>
    </span>
  );
}

export default function TotpGenerator() {
  const toast = useToast();
  // The form is a draft, so an account being typed in survives the popover closing.
  const [account, setAccount] = useDraft("totp:account", "");
  const [issuer, setIssuer] = useDraft("totp:issuer", "");
  const [secret, setSecret] = useDraft("totp:secret", "");
  const [algorithm, setAlgorithm] = useDraft<TotpAlgorithm>("totp:algorithm", "SHA1");
  const [digits, setDigits] = useDraft("totp:digits", 6);
  const [period, setPeriod] = useDraft("totp:period", 30);
  const [reveal, setReveal] = useState(false);
  const [accounts, setAccounts] = useState<TotpUri[] | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [removing, setRemoving] = useState<number | null>(null);
  const [qrError, setQrError] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const fileInput = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .totpLoadAccounts()
      .then((list) => !cancelled && setAccounts(list))
      .catch((e) => !cancelled && setLoadError(toToolFailure(e).display));
    return () => {
      cancelled = true;
    };
  }, []);

  const saved = accounts ?? [];
  const draftSecret = cleanSecret(secret);
  const codes = useTotpCodes(saved);
  // Separate from the saved codes, so typing the secret does not reset their timer.
  const [preview] = useTotpCodes(draftSecret ? [{ secret: draftSecret, algorithm, digits, period }] : []);

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

  const copyCode = async (code: TotpCode | null | undefined) => {
    if (!code) return;
    try {
      await copyText(code.code);
      toast("Code copied to clipboard");
    } catch (e) {
      toast(`Copy failed: ${String(e)}`);
    }
  };

  const store = async (next: TotpUri[]): Promise<boolean> => {
    try {
      await api.totpSaveAccounts(next);
      setAccounts(next);
      return true;
    } catch (e) {
      toast(`Could not save accounts: ${toToolFailure(e).display}`);
      return false;
    }
  };

  const saveAccount = async () => {
    // Without the stored list, saving would overwrite the accounts that failed to load.
    if (!accounts || !preview?.code) return;
    const entry: TotpUri = { account: account.trim(), issuer: issuer.trim(), secret: draftSecret, algorithm, digits, period };
    if (!(await store([...accounts, entry]))) return;
    setAccount("");
    setIssuer("");
    setSecret("");
    setAlgorithm("SHA1");
    setDigits(6);
    setPeriod(30);
    setReveal(false);
    toast("Account saved");
  };

  const removeAccount = async (index: number) => {
    setRemoving(null);
    if (accounts) await store(accounts.filter((_, i) => i !== index));
  };

  const setPeriodClamped = (n: number) => {
    if (Number.isFinite(n)) setPeriod(Math.min(300, Math.max(1, Math.trunc(n))));
  };

  const label = (a: TotpUri) => a.issuer || a.account || "Unnamed account";

  return (
    <div className="totp scroll">
      <h3 className="section-title">
        <KeyRound size={16} aria-hidden="true" /> SAVED ACCOUNTS
      </h3>
      {loadError && (
        <div className="error-box" role="alert">
          <div>
            <strong>Cannot load saved accounts</strong>
            <p>{loadError}</p>
          </div>
        </div>
      )}
      {accounts && accounts.length === 0 && (
        <p className="totp-empty muted">No saved accounts yet. Add one below to keep its code here.</p>
      )}
      {saved.length > 0 && (
        <ul className="totp-list">
          {saved.map((a, i) => {
            const result = codes[i];
            return (
              <li key={i} className="totp-account">
                <div className="totp-name">
                  <strong>{label(a)}</strong>
                  {a.issuer && a.account && <span className="muted">{a.account}</span>}
                </div>
                {result?.error && <span className="totp-error">{result.error}</span>}
                {result?.code && (
                  <>
                    <button className="totp-code" onClick={() => void copyCode(result.code)} title="Copy code">
                      {groupCode(result.code.code)}
                    </button>
                    <Ring remaining={result.code.remainingSeconds} period={a.period} size={34} />
                  </>
                )}
                <button
                  className="icon-btn"
                  onClick={() => setRemoving(i)}
                  aria-label={`Remove ${label(a)}`}
                  title="Remove account"
                >
                  <Trash2 size={16} />
                </button>
              </li>
            );
          })}
        </ul>
      )}

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

      {preview?.error && (
        <div className="error-box" role="alert">
          <div>
            <strong>Error</strong>
            <p>{preview.error}</p>
          </div>
        </div>
      )}
      {preview?.code && (
        <>
          <button className="code-card" onClick={() => void copyCode(preview.code)} title="Copy code">
            <span className="code">{groupCode(preview.code.code)}</span>
            <Ring remaining={preview.code.remainingSeconds} period={period} size={52} />
          </button>
          <div className="totp-save">
            <button className="primary" disabled={!accounts} onClick={() => void saveAccount()}>
              <Plus size={16} aria-hidden="true" /> Save Account
            </button>
          </div>
        </>
      )}

      {removing !== null && saved[removing] && (
        <ConfirmDialog
          title="Remove Account"
          message={`Remove "${label(saved[removing])}"? Its secret is deleted and cannot be recovered.`}
          confirmLabel="Remove"
          onConfirm={() => void removeAccount(removing)}
          onCancel={() => setRemoving(null)}
        />
      )}
    </div>
  );
}
