// Numbers headings, figures, tables and equations the way the PDF will, so the
// outline, the cross-reference picker and references in the text agree with it.
import type { Node as PMNode } from "@tiptap/pm/model";
import type { Template } from "../api";

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
  pos: number;
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
  if (level === 1 && appendix) return "Appendix";
  return level === 1 && t.structure.topLevel === "chapter" ? "Chapter" : "Section";
}

export function computeNumbering(doc: PMNode, t: Template): NumberedItem[] {
  const items: NumberedItem[] = [];
  const counters = [0, 0, 0, 0];
  let figure = 0;
  let table = 0;
  let equation = 0;
  let appendix = false;
  const chapters = t.structure.topLevel === "chapter";
  const perChapter = chapters && t.structure.perChapterNumbering;
  const top = () => (appendix ? letter(counters[0]) : String(counters[0]));
  const prefixed = (n: number) => (perChapter ? `${top()}.${n}` : String(n));

  doc.descendants((node, pos) => {
    const name = node.type.name;
    if (name === "appendixStart") {
      // Like LaTeX's \appendix: chapter numbering restarts (as letters); figure,
      // table and equation numbers only restart when they're per chapter anyway.
      appendix = true;
      counters.fill(0);
      return false;
    }
    if (name === "heading") {
      const level = Math.min(4, Math.max(1, node.attrs.level as number));
      counters[level - 1]++;
      for (let i = level; i < 4; i++) counters[i] = 0;
      if (level === 1 && perChapter) figure = table = equation = 0;
      const numbered = level <= t.structure.numberedDepth || (appendix && level === 1);
      const number = numbered ? [top(), ...counters.slice(1, level)].join(".") : "";
      items.push({
        id: node.attrs.id ?? null,
        kind: "heading",
        level,
        number,
        refText: number ? `${headingWord(t, level, appendix)} ${number}` : node.textContent,
        text: node.textContent || "(untitled)",
        appendix,
        pos,
      });
      return false;
    }
    if (name === "figure" || name === "tableFigure") {
      const isFigure = name === "figure";
      const number = prefixed(isFigure ? ++figure : ++table);
      items.push({
        id: node.attrs.id ?? null,
        kind: isFigure ? "figure" : "table",
        level: 5,
        number,
        refText: `${isFigure ? t.captions.figureLabel : t.captions.tableLabel} ${number}`,
        text: node.firstChild?.textContent || "(no caption)",
        appendix,
        pos,
      });
      return false;
    }
    if (name === "equation") {
      const number = prefixed(++equation);
      items.push({
        id: node.attrs.id ?? null,
        kind: "equation",
        level: 5,
        number,
        refText: `(${number})`,
        text: node.attrs.latex || "(empty equation)",
        appendix,
        pos,
      });
      return false;
    }
    return true;
  });
  return items;
}
