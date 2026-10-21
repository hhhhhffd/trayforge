// Keyboard shortcuts handled in App are forwarded to the active standard tool view.
export type ToolAction = "process" | "copy" | "clear" | "swap";

const EVENT = "trayforge:action";

export function emitAction(action: ToolAction) {
  window.dispatchEvent(new CustomEvent<ToolAction>(EVENT, { detail: action }));
}

export function onAction(handler: (action: ToolAction) => void): () => void {
  const listener = (e: Event) => handler((e as CustomEvent<ToolAction>).detail);
  window.addEventListener(EVENT, listener);
  return () => window.removeEventListener(EVENT, listener);
}
