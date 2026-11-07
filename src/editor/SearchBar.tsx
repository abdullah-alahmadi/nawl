// The find and replace bar over the editor (Ctrl+F, Ctrl+H for replace).
import { useEffect, useRef, useState } from "react";
import { useEditorState, type Editor } from "@tiptap/react";
import { ChevronDown, ChevronUp, Replace, X } from "lucide-react";
import { replaceAll, replaceCurrent, searchState, setSearch, stepMatch } from "./Search";
import { useT } from "../i18n";

export function SearchBar({ editor, withReplace, onClose }: { editor: Editor; withReplace: boolean; onClose: () => void }) {
  const t = useT();
  // Start with the selected text, if it's a few words on one line.
  const [query, setQuery] = useState(() => {
    const { from, to } = editor.state.selection;
    const text = editor.state.doc.textBetween(from, to, "\n");
    return text && text.length < 80 && !text.includes("\n") ? text : searchState(editor).query;
  });
  const [replacement, setReplacement] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [showReplace, setShowReplace] = useState(withReplace);
  const [notice, setNotice] = useState<string | null>(null);
  const input = useRef<HTMLInputElement>(null);
  const s = useEditorState({ editor, selector: ({ editor }) => {
    const st = searchState(editor);
    return { count: st.matches.length, current: st.current };
  } });

  useEffect(() => setShowReplace((v) => v || withReplace), [withReplace]);
  useEffect(() => {
    input.current?.focus();
    input.current?.select();
  }, [withReplace]);
  useEffect(() => {
    setSearch(editor, query, caseSensitive);
    setNotice(null);
  }, [editor, query, caseSensitive]);
  // Closing clears the marks.
  useEffect(() => () => setSearch(editor, "", false), [editor]);

  const close = () => {
    onClose();
    editor.commands.focus();
  };
  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "Enter" && (e.target as HTMLElement).dataset.field === "find") {
      e.preventDefault();
      stepMatch(editor, e.shiftKey ? -1 : 1);
    } else if (e.key === "Enter" && (e.target as HTMLElement).dataset.field === "replace") {
      e.preventDefault();
      if (e.ctrlKey || e.metaKey) setNotice(t("editor.replacedCount", { n: replaceAll(editor, replacement) }));
      else replaceCurrent(editor, replacement);
    }
  };

  const status = !query ? "" : s.count === 0 ? t("editor.noMatches") : t("editor.matchOf", { current: s.current + 1, count: s.count });
  return (
    <div className="search-bar" role="search" onKeyDown={onKey}>
      <div className="search-row">
        <button
          type="button"
          className={`icon-button small${showReplace ? " active" : ""}`}
          onClick={() => setShowReplace((v) => !v)}
          aria-label={t("editor.replace")}
          title={t("editor.replaceTitle")}
        >
          <Replace size={14} />
        </button>
        <input
          ref={input}
          data-field="find"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t("editor.findPlaceholder")}
          aria-label={t("editor.find")}
          spellCheck={false}
        />
        <span className={`search-count${query && s.count === 0 ? " none" : ""}`} aria-live="polite">
          {status}
        </span>
        <button
          type="button"
          className={`search-case${caseSensitive ? " active" : ""}`}
          onClick={() => setCaseSensitive((v) => !v)}
          aria-pressed={caseSensitive}
          title={t("editor.matchCase")}
        >
          Aa
        </button>
        <button type="button" className="icon-button small" onClick={() => stepMatch(editor, -1)} disabled={!s.count} aria-label={t("editor.previousMatch")} title={t("editor.previousTitle")}>
          <ChevronUp size={15} />
        </button>
        <button type="button" className="icon-button small" onClick={() => stepMatch(editor, 1)} disabled={!s.count} aria-label={t("editor.nextMatch")} title={t("editor.nextTitle")}>
          <ChevronDown size={15} />
        </button>
        <button type="button" className="icon-button small" onClick={close} aria-label={t("common.close")} title={t("editor.closeEsc")}>
          <X size={15} />
        </button>
      </div>
      {showReplace && (
        <div className="search-row">
          <span className="search-spacer" />
          <input
            data-field="replace"
            value={replacement}
            onChange={(e) => setReplacement(e.target.value)}
            placeholder={t("editor.replaceWith")}
            aria-label={t("editor.replaceWith")}
            spellCheck={false}
          />
          <button type="button" className="tb-button" onClick={() => replaceCurrent(editor, replacement)} disabled={!s.count} title={t("editor.replaceThisTitle")}>
            {t("editor.replace")}
          </button>
          <button type="button" className="tb-button" onClick={() => setNotice(t("editor.replacedCount", { n: replaceAll(editor, replacement) }))} disabled={!s.count} title={t("editor.replaceAllTitle")}>
            {t("editor.replaceAll")}
          </button>
          {notice && <span className="search-count">{notice}</span>}
        </div>
      )}
    </div>
  );
}
