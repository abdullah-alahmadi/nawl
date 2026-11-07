// Popovers for inserting cross-references and citations, and the References
// manager (the document's BibTeX).
import { useEffect, useMemo, useRef, useState } from "react";
import type { Editor } from "@tiptap/react";
import { open } from "@tauri-apps/plugin-dialog";
import * as api from "../api";
import type { BibEntry, BibInfo } from "../api";
import { useNumbering } from "./Academic";
import { ReferenceDialog } from "./ReferenceDialog";
import { appendEntry, replaceEntry } from "./bibtex";
import { useT } from "../i18n";

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
  const t = useT();
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
        <button onClick={onClose} aria-label={t("common.close")}>
          ×
        </button>
      </div>
      <input ref={input} value={q} onChange={(e) => setQ(e.target.value)} placeholder={t("editor.searchPlaceholder")} />
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
  const t = useT();
  const items = useNumbering(editor).filter((i) => i.id && (i.kind !== "heading" || i.number));
  return (
    <Popover
      title={t("editor.insertCrossRef")}
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
      empty={t("editor.crossRefEmpty")}
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
  const t = useT();
  return (
    <Popover
      title={t("editor.insertCitation")}
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
      empty={t("editor.citationEmpty")}
      footer={
        <button
          onClick={() => {
            onClose();
            onManage();
          }}
        >
          {t("editor.manageReferences")}
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
}: {
  source: string;
  info: BibInfo;
  cited: string[];
  onChange: (source: string) => void;
}) {
  const t = useT();
  const [problem, setProblem] = useState<string | null>(null);
  const [editing, setEditing] = useState(false);
  /** The reference window: a new entry, or the key of one being edited. */
  const [dialog, setDialog] = useState<{ key: string | null } | null>(null);
  const editingEntry = dialog?.key ? info.entries.find((e) => e.key === dialog.key) : undefined;
  const taken = new Set(info.entries.map((e) => e.key).filter((k) => k !== dialog?.key));
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
    <div className="refs">
      <p className="muted refs-help">
        {t("editor.refsHelp")}
      </p>
      <div className="refs-actions">
        <button className="tb-button primary" onClick={() => setDialog({ key: null })} disabled={!!info.error} title={info.error ? t("editor.fixBibtexFirst") : undefined}>
          {t("editor.newReference")}
        </button>
        <button className="tb-button" onClick={importFile}>
          {t("editor.importBib")}
        </button>
        <button className={`tb-button${editing ? " active" : ""}`} onClick={() => setEditing((v) => !v)}>
          {editing ? t("editor.hideBibtex") : t("editor.editBibtex")}
        </button>
      </div>
      {problem && <div className="d-error">{problem}</div>}
      {editing && (
        <textarea
          className="refs-source"
          spellCheck={false}
          value={source}
          rows={12}
          onChange={(e) => onChange(e.target.value)}
          placeholder={"@article{smith2021,\n  author = {Smith, Jane},\n  title = {...},\n  year = {2021}\n}"}
        />
      )}
      {info.error ? (
        <div className="d-error">{info.error}</div>
      ) : (
        <div className="refs-count muted">
          {info.entries.length === 1
            ? t("editor.refsCountOne", { cited: cited.length })
            : t("editor.refsCount", { n: info.entries.length, cited: cited.length })}
        </div>
      )}
      <div className="refs-entries">
        {info.entries.map((e) => (
          <button
            key={e.key}
            className={`refs-entry${cited.includes(e.key) ? " cited" : ""}`}
            title={cited.includes(e.key) ? t("editor.citedInText") : t("editor.notCitedYet")}
            onClick={() => setDialog({ key: e.key })}
          >
            <code>{e.key}</code>
            <span>
              {e.authors.join(", ")} {e.year && `(${e.year})`}
            </span>
            <span className="muted">{e.title}</span>
          </button>
        ))}
      </div>
      {dialog && (
        <ReferenceDialog
          entry={editingEntry}
          taken={taken}
          cited={!!editingEntry && cited.includes(editingEntry.key)}
          onSave={(text) => {
            onChange(editingEntry ? replaceEntry(source, editingEntry.key, text) : appendEntry(source, text));
            setDialog(null);
          }}
          onDelete={editingEntry ? () => {
            onChange(replaceEntry(source, editingEntry.key, null));
            setDialog(null);
          } : undefined}
          onClose={() => setDialog(null)}
        />
      )}
    </div>
  );
}
