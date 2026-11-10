// Numbers headings, figures, tables and equations the way the PDF will, so the
// outline, the cross-reference picker and references in the text agree with it.
import type { Node as PMNode } from "@tiptap/pm/model";
import type { ChapterIndex, Template } from "../api";
import { docLanguage, localDigits, words, abjad } from "./language";

export type TargetKind = "heading" | "figure" | "table" | "equation";

export interface NumberedItem {
  id: string | null;
  kind: TargetKind;
  /** Heading level 1–4 (5 for figures, tables, equations). */
  level: number;
  /** "3.1", "A", "A.2"; empty for unnumbered headings. */
  number: string;
  /** What a reference shows: "Figure 3.1", "Chapter 2", "(3.2)", "Appendix A". */
  refText: string;
  /** Title, caption or equation source, for lists. */
  text: string;
  appendix: boolean;
  /** Position in the editor's document; -1 for items of other chapters. */
  pos: number;
  /** In a project: the chapter file of an item in another chapter. */
  chapter?: string;
}

const letter = (n: number) => {
  let s = "";
  for (let x = n; x > 0; x = Math.floor((x - 1) / 26)) s = String.fromCharCode(65 + ((x - 1) % 26)) + s;
  return s;
};

/** "Chapter" from "Chapter {n}"; a default when the label is only the number. */
function headingWord(t: Template, level: number, appendix: boolean): string {
  const label = appendix && level === 1 ? t.structure.appendixLabel : t.headings.levels[level - 1]?.label ?? "";
  const word = label.replace(/\{n\}/g, "").replace(/^[^\p{L}\p{N}]+|[^\p{L}\p{N}]+$/gu, "");
  if (word) return word;
  const w = words(docLanguage(t).main);
  if (level === 1 && appendix) return w.appendix;
  return level === 1 && t.structure.topLevel === "chapter" ? w.chapter : w.section;
}

/** Where numbering stands: after the chapters before the open one, in a project. */
export interface Counters {
  /** Chapter, section, subsection, subsubsection. */
  levels: number[];
  figure: number;
  table: number;
  equation: number;
  appendix: boolean;
}

export const freshCounters = (): Counters => ({ levels: [0, 0, 0, 0], figure: 0, table: 0, equation: 0, appendix: false });

/** One thing that takes (or could take) a number, in document order. */
export type NumberEvent =
  | { kind: "appendix" }
  | { kind: "heading"; level: number; numbered: boolean; id: string | null; text: string; pos: number }
  | { kind: "figure" | "table"; id: string | null; text: string; pos: number }
  | { kind: "equation"; numbered: boolean; id: string | null; text: string; pos: number };

/** The document's numbered items. `start`: the counters after the chapters before
 *  this one, for a chapter of a project (left unchanged). */
export function computeNumbering(doc: PMNode, t: Template, start?: Counters): NumberedItem[] {
  const events: NumberEvent[] = [];
  doc.descendants((node, pos) => {
    const name = node.type.name;
    const id = (node.attrs.id as string | null | undefined) ?? null;
    if (name === "appendixStart") events.push({ kind: "appendix" });
    else if (name === "heading") events.push({ kind: "heading", level: node.attrs.level as number, numbered: node.attrs.numbered !== false, id, text: node.textContent, pos });
    else if (name === "figure" || name === "tableFigure") events.push({ kind: name === "figure" ? "figure" : "table", id, text: node.firstChild?.textContent ?? "", pos });
    else if (name === "equation") events.push({ kind: "equation", numbered: node.attrs.numbered !== false, id, text: node.attrs.latex as string, pos });
    else return true;
    return false;
  });
  return numberEvents(events, t, start ? structuredClone(start) : freshCounters());
}

/** Numbers `events` the way the PDF will, advancing `c`. */
export function numberEvents(events: NumberEvent[], t: Template, c: Counters): NumberedItem[] {
  // Numbers written with the document's digits (٣٫١ in Arabic documents that use them).
  const { digits, rtl } = docLanguage(t);
  // In a right-to-left document each part of a number is set on its own, the chapter
  // on the right (2.1 reads "١.٢"), as the PDF does: right-to-left marks around the
  // dot keep the parts from joining into one left-to-right number.
  const RLM = String.fromCharCode(0x200f);
  const parts = (s: string) => (rtl ? s.replace(/(\S)\.(?=\S)/g, "$1" + RLM + "." + RLM) : s);
  const local = (s: string) => parts(digits === "western" ? s : localDigits(s, digits));
  return numberWestern(events, t, c).map((i) => (i.number ? { ...i, number: local(i.number), refText: local(i.refText) } : i));
}

function numberWestern(events: NumberEvent[], t: Template, c: Counters): NumberedItem[] {
  const items: NumberedItem[] = [];
  const counters = c.levels;
  const chapters = t.structure.topLevel === "chapter";
  const perChapter = chapters && t.structure.perChapterNumbering;
  // Appendix letters: abjad letters in Arabic documents, as polyglossia prints them.
  const arabic = docLanguage(t).main === "arabic";
  const top = () => (c.appendix ? (arabic ? abjad(counters[0]) : letter(counters[0])) : String(counters[0]));
  const prefixed = (n: number) => (perChapter ? `${top()}.${n}` : String(n));

  for (const e of events) {
    if (e.kind === "appendix") {
      // Like LaTeX's \appendix: chapter numbering restarts (as letters); figure,
      // table and equation numbers only restart when they're per chapter anyway.
      c.appendix = true;
      counters.fill(0);
      continue;
    }
    const appendix = c.appendix;
    if (e.kind === "heading") {
      const level = Math.min(4, Math.max(1, e.level));
      if (!e.numbered) {
        // Like \chapter*: no number, and the counters carry on as if it weren't there.
        items.push({ id: e.id, kind: "heading", level, number: "", refText: e.text, text: e.text || "(untitled)", appendix, pos: e.pos });
        continue;
      }
      counters[level - 1]++;
      for (let i = level; i < 4; i++) counters[i] = 0;
      if (level === 1 && perChapter) c.figure = c.table = c.equation = 0;
      const numbered = level <= t.structure.numberedDepth || (appendix && level === 1);
      const number = numbered ? [top(), ...counters.slice(1, level)].join(".") : "";
      items.push({
        id: e.id,
        kind: "heading",
        level,
        number,
        refText: number ? `${headingWord(t, level, appendix)} ${number}` : e.text,
        text: e.text || "(untitled)",
        appendix,
        pos: e.pos,
      });
    } else if (e.kind === "figure" || e.kind === "table") {
      const isFigure = e.kind === "figure";
      const number = prefixed(isFigure ? ++c.figure : ++c.table);
      items.push({
        id: e.id,
        kind: e.kind,
        level: 5,
        number,
        refText: `${isFigure ? t.captions.figureLabel : t.captions.tableLabel} ${number}`,
        text: e.text || "(no caption)",
        appendix,
        pos: e.pos,
      });
    } else if (e.kind === "equation" && e.numbered) {
      const number = prefixed(++c.equation);
      items.push({ id: e.id, kind: "equation", level: 5, number, refText: `(${number})`, text: e.text || "(empty equation)", appendix, pos: e.pos });
    }
  }
  return items;
}

/** For a chapter of a project: where numbering starts (after the chapters before it),
 *  and the numbered items of the other chapters (references to them show their
 *  numbers; `chapter` says where they are). Chapters come from the project index. */
export function bookNumbering(chapters: ChapterIndex[], openFile: string, t: Template): { start: Counters; others: NumberedItem[] } {
  const c = freshCounters();
  let start = freshCounters();
  const others: NumberedItem[] = [];
  let appendix = false;
  for (const ch of chapters) {
    if (ch.appendix && !appendix) {
      appendix = true;
      numberEvents([{ kind: "appendix" }], t, c);
    }
    if (ch.file === openFile) {
      start = structuredClone(c);
      // The open chapter's own items come from the editor; skip past them here.
      numberEvents(eventsOf(ch), t, c);
      continue;
    }
    others.push(...numberEvents(eventsOf(ch), t, c).map((i) => ({ ...i, pos: -1, chapter: ch.file })));
  }
  return { start, others };
}

function eventsOf(ch: ChapterIndex): NumberEvent[] {
  return ch.targets.map((x): NumberEvent => {
    switch (x.kind) {
      case "heading":
        return { kind: "heading", level: x.level, numbered: x.numbered, id: x.id, text: x.text, pos: -1 };
      case "equation":
        return { kind: "equation", numbered: x.numbered, id: x.id, text: x.text, pos: -1 };
      default:
        return { kind: x.kind, id: x.id, text: x.text, pos: -1 };
    }
  });
}
