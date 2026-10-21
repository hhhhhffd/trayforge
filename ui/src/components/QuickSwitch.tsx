import { useEffect, useMemo, useRef, useState } from "react";
import { CornerDownLeft, Search } from "lucide-react";
import { alphabetical, searchTools } from "../lib";
import type { ToolInfo } from "../types";
import { Icon } from "./Icon";

export function QuickSwitch({
  tools,
  onClose,
  onOpenTool,
}: {
  tools: ToolInfo[];
  onClose: () => void;
  onOpenTool: (id: string) => void;
}) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const items = useMemo(
    () => (query.trim() ? searchTools(tools, query) : alphabetical(tools)),
    [tools, query],
  );

  useEffect(() => {
    listRef.current?.children[selected]?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, items.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
    } else if (e.key === "Enter" && items[selected]) {
      onOpenTool(items[selected].id);
    }
  };

  return (
    <div className="overlay" onMouseDown={onClose} data-overlay>
      <div
        className="qs"
        role="dialog"
        aria-label="Quick Switch"
        onMouseDown={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <div className="quick-search">
          <Search size={20} aria-hidden="true" />
          <input
            autoFocus
            value={query}
            placeholder="Search tools..."
            aria-label="Search tools"
            spellCheck={false}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelected(0);
            }}
          />
          <kbd>esc</kbd>
        </div>
        <div className="quick-list" ref={listRef} role="listbox">
          {items.map((t, i) => (
            <button
              key={t.id}
              role="option"
              aria-selected={i === selected}
              className={`quick-item${i === selected ? " selected" : ""}`}
              onMouseMove={() => setSelected(i)}
              onClick={() => onOpenTool(t.id)}
            >
              <span className="quick-icon">
                <Icon name={t.icon} size={22} />
              </span>
              <span className="quick-text">
                <strong>{t.name}</strong>
                <small>{t.description}</small>
              </span>
              {i === selected && <CornerDownLeft size={16} aria-hidden="true" />}
            </button>
          ))}
          {items.length === 0 && <div className="side-empty">No tools found</div>}
        </div>
      </div>
    </div>
  );
}
