// Unsaved panel input. The window is destroyed on every close, so drafts live in the tray
// process (memory only, never on disk) and come back when the window reopens.
import { useEffect, useState, type Dispatch, type SetStateAction } from "react";
import { api } from "./api";

const SAVE_DEBOUNCE_MS = 300;

let drafts: Record<string, unknown> = {};
const pending = new Set<string>();
let timer: number | undefined;

/** Must finish before the first render that reads drafts. */
export async function loadDrafts(): Promise<void> {
  drafts = await api.getDrafts();
}

export function readDraft<T>(key: string, fallback: T): T {
  return Object.hasOwn(drafts, key) ? (drafts[key] as T) : fallback;
}

export function writeDraft(key: string, value: unknown): void {
  if (Object.is(drafts[key], value)) return;
  drafts[key] = value;
  pending.add(key);
  window.clearTimeout(timer);
  timer = window.setTimeout(() => void flushDrafts(), SAVE_DEBOUNCE_MS);
}

/** Sends pending drafts to the shell; await before the window is destroyed. */
export async function flushDrafts(): Promise<void> {
  window.clearTimeout(timer);
  const keys = [...pending];
  pending.clear();
  await Promise.all(
    keys.map((key) => api.setDraft(key, drafts[key] ?? null).catch((e) => console.error("set_draft failed", key, e))),
  );
}

/** `useState` that survives the window being closed and reopened. */
export function useDraft<T>(key: string, fallback: T): [T, Dispatch<SetStateAction<T>>] {
  const [value, setValue] = useState<T>(() => readDraft(key, fallback));
  useEffect(() => writeDraft(key, value), [key, value]);
  return [value, setValue];
}
