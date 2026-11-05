// Popovers for inserting cross-references and citations, and the References
// manager (the document's BibTeX).
import { useEffect, useMemo, useRef, useState } from "react";
import type { Editor } from "@tiptap/react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { BibEntry, BibInfo } from "../api";
import { useNumbering } from "./Academic";

/** A popover with a search box and a keyboard-navigable list. */
function Popover<T>({
  title,
  items,
  render,
  match,
  onPick,
  onClose,
  empty,
  footer,
}: {
  title: string;
  items: T[];
  render: (item: T) => React.ReactNode;
  match: (item: T, q: string) => boolean;
  onPick: (item: T) => void;
  onClose: () => void;
  empty: React.ReactNode;
  footer?: React.ReactNode;
}) {
  const [q, setQ] = useState("");
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const shown = useMemo(() => items.filter((i) => match(i, q.toLowerCase())), [items, q, match]);
  useEffect(() => input.current?.focus(), []);
  useEffect(() => setActive(0), [q]);
  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") onClose();
    else if (e.key === "ArrowDown") setActive((a) => Math.min(shown.length - 1, a + 1));
    else if (e.key === "ArrowUp") setActive((a) => Math.max(0, a - 1));
    else if (e.key === "Enter" && shown[active]) onPick(shown[active]);
    else return;
    e.preventDefault();
  };
  return (
    <div className="picker" role="dialog" aria-label={title} onKeyDown={onKey}>
      <div className="picker-head">
        <strong>{title}</strong>
        <button onClick={onClose} aria-label="Close">
          ×
        </button>
      </div>
      <input ref={input} value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search…" />
      <div className="picker-list" role="listbox">
        {items.length === 0 && <div className="picker-empty">{empty}</div>}
        {shown.map((item, i) => (
          <button
            key={i}
            role="option"
            aria-selected={i === active}
            className={i === active ? "active" : ""}
            onMouseEnter={() => setActive(i)}
            onClick={() => onPick(item)}
          >
            {render(item)}
          </button>
        ))}
      </div>
      {footer && <div className="picker-foot">{footer}</div>}
    </div>
  );
}

export function RefPicker({ editor, onClose }: { editor: Editor; onClose: () => void }) {
  const items = useNumbering(editor).filter((i) => i.id && (i.kind !== "heading" || i.number));
  return (
    <Popover
      title="Insert cross-reference"
      items={items}
      match={(i, q) => !q || `${i.refText} ${i.text}`.toLowerCase().includes(q)}
      render={(i) => (
        <>
          <span className="picker-main">{i.refText}</span>
          <span className="picker-sub">{i.kind === "equation" ? i.text : i.text}</span>
        </>
      )}
      onPick={(i) => {
        editor.chain().focus().insertRef(i.id!).run();
        onClose();
      }}
      onClose={onClose}
      empty="Numbered headings, figures, tables and equations appear here."
    />
  );
}

export function CitePicker({
  editor,
  entries,
  onClose,
  onManage,
}: {
  editor: Editor;
  entries: BibEntry[];
  onClose: () => void;
  onManage: () => void;
}) {
  return (
    <Popover
      title="Insert citation"
      items={entries}
      match={(e, q) => !q || `${e.key} ${e.authors.join(" ")} ${e.year ?? ""} ${e.title}`.toLowerCase().includes(q)}
      render={(e) => (
        <>
          <span className="picker-main">
            {e.authors.length ? e.authors.slice(0, 2).join(", ") + (e.authors.length > 2 ? " et al." : "") : e.key}
            {e.year && ` (${e.year})`}
          </span>
          <span className="picker-sub">{e.title}</span>
        </>
      )}
      onPick={(e) => {
        editor.chain().focus().insertCitation(e.key).run();
        onClose();
      }}
      onClose={onClose}
      empty={<>No references yet. Add them in References.</>}
      footer={
        <button
          onClick={() => {
            onClose();
            onManage();
          }}
        >
          Manage references…
        </button>
      }
    />
  );
}

/** The document's bibliography: BibTeX source, import, and what was parsed. */
export function ReferencesPanel({
  source,
  info,
  cited,
  onChange,
  onClose,
}: {
  source: string;
  info: BibInfo;
  cited: string[];
  onChange: (source: string) => void;
  onClose: () => void;
}) {
  const [problem, setProblem] = useState<string | null>(null);
  const importFile = async () => {
    setProblem(null);
    const file = await open({ multiple: false, directory: false, filters: [{ name: "BibTeX", extensions: ["bib"] }] });
    if (!file) return;
    try {
      const text = await api.readBibFile(file);
      onChange(source.trim() ? `${source.trimEnd()}\n\n${text}` : text);
    } catch (e) {
      setProblem(String((api.asAppError(e) as { message?: string }).message ?? e));
    }
  };
  return (
    <div className="refs-panel" role="dialog" aria-label="References">
      <div className="refs-head">
        <div>
          <h2>References</h2>
          <p className="muted">
            BibTeX entries for this document. Export them from Zotero, Mendeley or Google Scholar (“BibTeX”), then
            import or paste them here.
          </p>
        </div>
        <div className="group">
          <button onClick={importFile}>Import .bib…</button>
          <button className="primary" onClick={onClose}>
            Done
          </button>
        </div>
      </div>
      {problem && <div className="d-error">{problem}</div>}
      <div className="refs-body">
        <textarea
          className="refs-source"
          spellCheck={false}
          value={source}
          onChange={(e) => onChange(e.target.value)}
          placeholder={"@article{smith2021,\n  author = {Smith, Jane},\n  title = {...},\n  year = {2021}\n}"}
        />
        <div className="refs-entries">
          {info.error ? (
            <div className="d-error">{info.error}</div>
          ) : (
            <div className="muted">
              {info.entries.length} entr{info.entries.length === 1 ? "y" : "ies"}, {cited.length} cited
            </div>
          )}
          {info.entries.map((e) => (
            <div key={e.key} className={`refs-entry${cited.includes(e.key) ? " cited" : ""}`}>
              <code>{e.key}</code>
              <span>
                {e.authors.join(", ")} {e.year && `(${e.year})`}
              </span>
              <span className="muted">{e.title}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
