import type { Category, ToolInfo } from "./types";

export const CATEGORY_INFO: Record<Category, { icon: string; blurb: string }> = {
  Encoding: { icon: "case-sensitive", blurb: "Encode and decode text in various formats" },
  Hashing: { icon: "hash", blurb: "Generate hashes and cryptographic values" },
  Formats: { icon: "file-text", blurb: "Format and convert data between formats" },
  "Web Dev": { icon: "globe", blurb: "Tools for web development" },
  Text: { icon: "text-align-start", blurb: "Text processing and manipulation" },
  "Dev Utils": { icon: "wrench", blurb: "Development utilities and helpers" },
  Graphics: { icon: "palette", blurb: "Colors, palettes and vector graphics" },
};

/** Ids of tools rendered by a dedicated panel (see panels/index.ts). */
export const SCREEN_COLOR_PICKER_ID = "screen-color-picker";

/** Tools of a category in the user's saved order; unsaved/new tools keep list order at the end. */
export function categoryTools(tools: ToolInfo[], category: Category, saved: string[] | undefined): ToolInfo[] {
  const inCategory = tools.filter((t) => t.category === category);
  const rank = (id: string) => {
    const i = saved?.indexOf(id) ?? -1;
    return i < 0 ? Number.MAX_SAFE_INTEGER : i;
  };
  return inCategory
    .map((tool, index) => ({ tool, index }))
    .sort((a, b) => rank(a.tool.id) - rank(b.tool.id) || a.index - b.index)
    .map((x) => x.tool);
}

/** Ranked substring search over name, id, keywords and description. */
export function searchTools(tools: ToolInfo[], query: string): ToolInfo[] {
  const q = query.trim().toLowerCase();
  if (!q) return tools;
  const ranked = [];
  for (const t of tools) {
    const name = t.name.toLowerCase();
    let rank = -1;
    if (name === q) rank = 0;
    else if (name.startsWith(q)) rank = 1;
    else if (name.includes(q)) rank = 2;
    else if (t.id.includes(q) || t.keywords.some((k) => k.toLowerCase().includes(q))) rank = 3;
    else if (t.description.toLowerCase().includes(q)) rank = 4;
    if (rank >= 0) ranked.push({ rank, tool: t });
  }
  return ranked.sort((a, b) => a.rank - b.rank).map((r) => r.tool);
}

export function alphabetical(tools: ToolInfo[]): ToolInfo[] {
  return [...tools].sort((a, b) => a.name.localeCompare(b.name, "en", { sensitivity: "base" }));
}

export function countChars(s: string): string {
  const n = [...s].length;
  return `${n} char${n === 1 ? "" : "s"}`;
}

/** Moves `list[index]` by `delta`, returning a new array (or the same one at the edges). */
export function moveItem<T>(list: T[], index: number, delta: number): T[] {
  const target = index + delta;
  if (target < 0 || target >= list.length) return list;
  const next = [...list];
  [next[index], next[target]] = [next[target], next[index]];
  return next;
}

/** Turns a keyboard event into a Tauri accelerator string, or null for a lone modifier. */
export function acceleratorFromEvent(e: KeyboardEvent): string | null {
  const keyMap: Record<string, string> = {
    " ": "Space",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    Escape: "Escape",
    Enter: "Enter",
    Tab: "Tab",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
  };
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return null;
  let key = keyMap[e.key];
  if (!key) {
    if (/^F\d{1,2}$/.test(e.key)) key = e.key;
    else if (e.code.startsWith("Key")) key = e.code.slice(3);
    else if (e.code.startsWith("Digit")) key = e.code.slice(5);
    else if (e.key.length === 1) key = e.key.toUpperCase();
    else return null;
  }
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"];
  return [...mods.filter(Boolean), key].join("+");
}
