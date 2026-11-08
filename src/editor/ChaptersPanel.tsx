// The Chapters panel of a project: the chapter list (parts, chapters, appendices),
// which chapter is open, and changes to the list. Changes go to `onEntries`; the app
// saves the project file.
import { useRef, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { ArrowDown, ArrowUp, BookOpen, FilePlus2, FolderOpen, FolderPlus, Paperclip, Pencil, Trash2, Undo2 } from "lucide-react";
import type { ChapterIndex, ProjectEntry, ProjectIndex } from "../api";
import { useT } from "../i18n";

type Chapter = Extract<ProjectEntry, { file: string }>;
const isChapter = (e: ProjectEntry): e is Chapter => "file" in e;
const isAppendix = (e: ProjectEntry) => isChapter(e) && !!e.appendix;

/** Moves entry `i` one place up (-1) or down (1), within its group: parts and
 *  chapters, or appendices (which always come last). */
export function moveEntry(entries: ProjectEntry[], i: number, dir: -1 | 1): ProjectEntry[] {
  const j = i + dir;
  if (j < 0 || j >= entries.length || isAppendix(entries[i]) !== isAppendix(entries[j])) return entries;
  const next = [...entries];
  [next[i], next[j]] = [next[j], next[i]];
  return next;
}

/** Makes chapter `i` an appendix (the first one) or a chapter again (the last one). */
export function toggleAppendix(entries: ProjectEntry[], i: number): ProjectEntry[] {
  const e = entries[i];
  if (!isChapter(e)) return entries;
  const rest = entries.filter((_, k) => k !== i);
  const boundary = rest.findIndex(isAppendix);
  const at = boundary < 0 ? rest.length : boundary;
  const moved: Chapter = e.appendix ? { file: e.file } : { file: e.file, appendix: true };
  return [...rest.slice(0, at), moved, ...rest.slice(at)];
}

/** Adds a part before the first appendix (at the end of the chapters). */
export function addPart(entries: ProjectEntry[], title: string): ProjectEntry[] {
  const boundary = entries.findIndex(isAppendix);
  const at = boundary < 0 ? entries.length : boundary;
  return [...entries.slice(0, at), { part: title }, ...entries.slice(at)];
}

export function ChaptersPanel({
  entries,
  index,
  current,
  onOpen,
  onEntries,
  onNewChapter,
}: {
  entries: ProjectEntry[];
  index: ProjectIndex;
  /** The chapter open in the editor. */
  current: string;
  onOpen: (file: string) => void;
  onEntries: (next: ProjectEntry[]) => void;
  onNewChapter: (appendix: boolean) => void;
}) {
  const t = useT();
  const byFile = new Map<string, ChapterIndex>(index.chapters.map((c) => [c.file, c]));
  const chapterCount = entries.filter(isChapter).length;
  const titleOf = (file: string) => byFile.get(file)?.title || t("project.untitledChapter");
  const firstAppendix = entries.findIndex(isAppendix);
  // The part whose title is being edited (a new part starts in editing).
  const [editing, setEditing] = useState<number | null>(null);

  const remove = async (i: number) => {
    const e = entries[i];
    if (isChapter(e) && !(await ask(t("project.removePrompt", { name: titleOf(e.file) }), { title: t("project.removeTitle"), kind: "warning" }))) return;
    onEntries(entries.filter((_, k) => k !== i));
  };

  let chapterNo = 0;
  let partNo = 0;
  return (
    <div className="chapters-panel">
      <ol className="chapter-list">
        {entries.map((e, i) => {
          const heading = i === firstAppendix ? <li key="appendices" className="chapter-group">{t("project.appendices")}</li> : null;
          const tools = (
            <span className="chapter-tools">
              <button className="icon-button small" onClick={() => onEntries(moveEntry(entries, i, -1))} disabled={moveEntry(entries, i, -1) === entries} title={t("project.moveUp")} aria-label={t("project.moveUp")}>
                <ArrowUp size={13} />
              </button>
              <button className="icon-button small" onClick={() => onEntries(moveEntry(entries, i, 1))} disabled={moveEntry(entries, i, 1) === entries} title={t("project.moveDown")} aria-label={t("project.moveDown")}>
                <ArrowDown size={13} />
              </button>
              {!isChapter(e) && (
                <button className="icon-button small" onClick={() => setEditing(i)} title={t("project.renamePart")} aria-label={t("project.renamePart")}>
                  <Pencil size={13} />
                </button>
              )}
              {isChapter(e) && (
                <button
                  className="icon-button small"
                  onClick={() => onEntries(toggleAppendix(entries, i))}
                  title={e.appendix ? t("project.makeChapter") : t("project.makeAppendix")}
                  aria-label={e.appendix ? t("project.makeChapter") : t("project.makeAppendix")}
                >
                  {e.appendix ? <Undo2 size={13} /> : <Paperclip size={13} />}
                </button>
              )}
              <button
                className="icon-button small"
                onClick={() => remove(i)}
                disabled={isChapter(e) && chapterCount <= 1}
                title={isChapter(e) ? t("project.remove") : t("project.removePart")}
                aria-label={isChapter(e) ? t("project.remove") : t("project.removePart")}
              >
                <Trash2 size={13} />
              </button>
            </span>
          );
          if (!isChapter(e)) {
            partNo++;
            return (
              <li key={`part-${i}`} className="chapter-part">
                {editing === i ? (
                  <PartTitle
                    value={e.part}
                    onDone={(part) => {
                      setEditing(null);
                      if (part !== null && part !== e.part) onEntries(entries.map((x, k) => (k === i ? { part } : x)));
                    }}
                  />
                ) : (
                  <span className="part-label" onDoubleClick={() => setEditing(i)} title={t("project.renamePartHint")}>
                    <FolderOpen size={14} aria-hidden="true" />
                    <span className="chapter-title">
                      <span className="chapter-no">{t("project.partNo", { n: partNo })}</span> {e.part || t("project.partPlaceholder")}
                    </span>
                  </span>
                )}
                {tools}
              </li>
            );
          }
          const info = byFile.get(e.file);
          const open = e.file === current;
          const inPart = !e.appendix && partNo > 0;
          if (!e.appendix) chapterNo++;
          return [
            heading,
            <li key={e.file} className={`chapter-row${open ? " open" : ""}${inPart ? " in-part" : ""}`}>
              <button className="chapter-open" onClick={() => onOpen(e.file)} title={open ? t("project.current") : `${t("project.open")}\n${e.file}`} aria-current={open || undefined}>
                <BookOpen size={14} aria-hidden="true" />
                <span className="chapter-title">
                  {!e.appendix && <span className="chapter-no">{chapterNo}.</span>} {titleOf(e.file)}
                </span>
                {info && <span className="chapter-words">{t("project.words", { n: info.words.toLocaleString() })}</span>}
              </button>
              {tools}
            </li>,
          ];
        })}
      </ol>
      <div className="chapter-add">
        <button className="tb-button" onClick={() => {
            const boundary = entries.findIndex(isAppendix);
            setEditing(boundary < 0 ? entries.length : boundary);
            onEntries(addPart(entries, t("project.newPartTitle")));
          }} title={t("project.addPartTitle")}>
          <FolderPlus size={14} aria-hidden="true" /> {t("project.addPart")}
        </button>
        <button className="tb-button" onClick={() => onNewChapter(false)} title={t("project.addChapterTitle")}>
          <FilePlus2 size={14} aria-hidden="true" /> {t("project.addChapter")}
        </button>
        <button className="tb-button" onClick={() => onNewChapter(true)} title={t("project.addAppendixTitle")}>
          <Paperclip size={14} aria-hidden="true" /> {t("project.addAppendix")}
        </button>
      </div>
    </div>
  );
}

/** A part's title being edited: Enter or leaving the field saves it, Escape cancels
 *  (`onDone(null)`). Starts focused with the text selected. */
function PartTitle({ value, onDone }: { value: string; onDone: (v: string | null) => void }) {
  const t = useT();
  const [text, setText] = useState(value);
  const done = useRef(false);
  const finish = (v: string | null) => {
    if (done.current) return;
    done.current = true;
    onDone(v === null ? null : v.trim() || value);
  };
  return (
    <input
      className="part-title"
      value={text}
      autoFocus
      onFocus={(e) => e.currentTarget.select()}
      placeholder={t("project.partPlaceholder")}
      aria-label={t("project.partPlaceholder")}
      onChange={(e) => setText(e.target.value)}
      onBlur={() => finish(text)}
      onKeyDown={(e) => {
        if (e.key === "Enter") finish(text);
        if (e.key === "Escape") finish(null);
      }}
    />
  );
}
