import { Suspense, useEffect, useRef, useState, type MutableRefObject } from "react";
import { HelpCircle, Star } from "lucide-react";
import { panels } from "../panels";
import type { ToolInfo } from "../types";
import { Icon } from "./Icon";
import { StandardTool } from "./StandardTool";

interface Props {
  tool: ToolInfo;
  isFavorite: boolean;
  onToggleFavorite: () => void;
  carriedInput: MutableRefObject<string | null>;
}

export function ToolPage({ tool, isFavorite, onToggleFavorite, carriedInput }: Props) {
  const [helpOpen, setHelpOpen] = useState(false);
  const helpRef = useRef<HTMLDivElement>(null);
  const Panel = tool.customUi ? panels[tool.id] : undefined;

  useEffect(() => {
    if (!helpOpen) return;
    const onDown = (e: MouseEvent) => {
      if (!helpRef.current?.contains(e.target as Node)) setHelpOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [helpOpen]);

  return (
    <div className="tool-page">
      <header className="tool-header" data-drag>
        <Icon name={tool.icon} size={36} className="tool-icon" />
        <div className="tool-title">
          <div className="tool-title-row">
            <h1>{tool.name}</h1>
            <span className="badge">{tool.category}</span>
          </div>
          <p title={tool.description}>{tool.description}</p>
        </div>
        <button
          className={`icon-btn big${isFavorite ? " on" : ""}`}
          onClick={onToggleFavorite}
          aria-pressed={isFavorite}
          aria-label={isFavorite ? "Remove from favorites" : "Add to favorites"}
          title={isFavorite ? "Remove from favorites (Ctrl+B)" : "Add to favorites (Ctrl+B)"}
        >
          <Star size={20} fill={isFavorite ? "currentColor" : "none"} />
        </button>
        <div
          className="help-wrap"
          ref={helpRef}
          onKeyDown={(e) => {
            if (e.key === "Escape" && helpOpen) {
              e.stopPropagation();
              setHelpOpen(false);
            }
          }}
        >
          <button
            className="icon-btn big"
            onClick={() => setHelpOpen((o) => !o)}
            aria-expanded={helpOpen}
            aria-label="Help"
            title="Show tool help"
          >
            <HelpCircle size={20} />
          </button>
          {helpOpen && (
            <div className="popover" role="dialog" aria-label={`${tool.name} help`}>
              <strong>{tool.name}</strong>
              <p>{tool.description}</p>
              {tool.keywords.length > 0 && (
                <div className="chips">
                  {tool.keywords.map((k) => (
                    <span key={k} className="chip">
                      {k}
                    </span>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>
      </header>
      <div className="tool-body">
        {tool.customUi ? (
          Panel ? (
            <Suspense fallback={<div className="loading">Loading…</div>}>
              <Panel tool={tool} />
            </Suspense>
          ) : (
            <div className="empty-state">No panel is available for this tool.</div>
          )
        ) : (
          <StandardTool tool={tool} carriedInput={carriedInput} />
        )}
      </div>
    </div>
  );
}
