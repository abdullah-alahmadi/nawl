// Turns a template into CSS for the editor page, so the page you type on looks like
// the PDF: margins, fonts, spacing, heading styles with their numbers ("CHAPTER 1",
// "3.1"), and figure/table/equation numbering. The PDF preview stays the exact version.
import type { Template } from "../api";
import type { FontChoice } from "../bindings/FontChoice";
import type { TextStyle } from "../bindings/TextStyle";

/** Width of the on-screen page in CSS pixels. */
export const PAGE_WIDTH_PX = 800;
const MM_PER_PT = 25.4 / 72.27;

export function cssFontStack(f: FontChoice): string {
  if (f.kind === "system") return `"${f.family.replace(/"/g, "")}", serif`;
  return {
    latinModern: '"Latin Modern Roman", "CMU Serif", serif',
    termes: '"TeX Gyre Termes", "Times New Roman", Times, serif',
    pagella: '"TeX Gyre Pagella", Palatino, "Palatino Linotype", serif',
    bonum: '"TeX Gyre Bonum", "Bookman Old Style", serif',
    schola: '"TeX Gyre Schola", "Century Schoolbook", serif',
    heros: '"TeX Gyre Heros", Helvetica, Arial, sans-serif',
    adventor: '"TeX Gyre Adventor", "Avant Garde", sans-serif',
    cursor: '"TeX Gyre Cursor", "Courier New", monospace',
  }[f.font];
}

function paperWidthMm(t: Template): number {
  const p = t.page.paper;
  return { a4: 210, a5: 148, letter: 215.9, legal: 215.9, custom: p.kind === "custom" ? p.widthMm : 210 }[p.kind];
}

function stretch(t: Template): number {
  const size = t.fonts.baseSizePt;
  switch (t.page.lineSpacing.kind) {
    case "single":
      return 1;
    case "oneHalf":
      return size === 10 ? 1.25 : size === 11 ? 1.213 : 1.241;
    case "double":
      return size === 10 ? 1.667 : size === 11 ? 1.618 : 1.655;
    case "custom":
      return t.page.lineSpacing.stretch;
  }
}

/** A CSS string literal. */
const q = (s: string) => `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, " ")}"`;

function textRules(s: TextStyle, px: (pt: number) => string): string {
  return [
    `font-size:${px(s.sizePt)}`,
    `font-weight:${s.bold ? 700 : 400}`,
    `font-style:${s.italic ? "italic" : "normal"}`,
    `font-variant:${s.smallCaps ? "small-caps" : "normal"}`,
    `text-transform:${s.uppercase ? "uppercase" : "none"}`,
    `text-align:${s.align}`,
    `color:${s.color ?? "inherit"}`,
  ].join(";");
}

/** CSS rules scoped to `scope` (the editor page element). */
export function templateCss(t: Template, scope = ".page"): string {
  const k = PAGE_WIDTH_PX / paperWidthMm(t); // px per mm
  const px = (pt: number) => `${(pt * MM_PER_PT * k).toFixed(2)}px`;
  const mm = (v: number) => `${(v * k).toFixed(1)}px`;
  const chapters = t.structure.topLevel === "chapter";
  const lineHeight = (1.2 * stretch(t)).toFixed(3);
  const inner = t.page.margins.innerMm + t.page.bindingOffsetMm;

  // Counter expression for heading level i (0-based): "1", "1.2", "1.2.3"…
  const counterExpr = (i: number) =>
    Array.from({ length: i + 1 }, (_, j) => `counter(tl-h${j + 1})`).join(' "." ');
  // Label template ("Chapter {n}") → CSS content value.
  const labelContent = (label: string, i: number) =>
    label
      .split(/(\{n\})/)
      .filter((part) => part !== "")
      .map((part) => (part === "{n}" ? counterExpr(i) : q(part)))
      .join(" ") || '""';

  const perChapter = chapters && t.structure.perChapterNumbering;
  const numberPrefix = perChapter ? 'counter(tl-h1) "." ' : "";
  const sep = { colon: ": ", period: ". ", dash: " — ", newline: "" }[t.captions.separator];
  const captionSize = { footnote: 0.8, small: 0.9, normal: 1 }[t.captions.size];
  const captionAlign = t.captions.align === "left" ? "left" : t.captions.align === "justified" ? "justify" : "center";

  const rules: string[] = [
    `${scope}{width:${PAGE_WIDTH_PX}px;max-width:none;padding:${mm(t.page.margins.topMm)} ${mm(t.page.margins.outerMm)} ${mm(t.page.margins.bottomMm)} ${mm(inner)};font-family:${cssFontStack(t.fonts.main)};font-size:${px(t.fonts.baseSizePt)};line-height:${lineHeight}}`,
    `${scope} .tiptap{counter-reset:tl-h1 tl-h2 tl-h3 tl-h4 tl-figure tl-table tl-equation}`,
    `${scope} p{margin:0;text-indent:${mm(t.page.paragraph.indentMm)};padding-top:${px(t.page.paragraph.spacingPt)}}`,
    `${scope} li p, ${scope} figcaption, ${scope} td p, ${scope} th p{text-indent:0;padding-top:0}`,
    `${scope} code, ${scope} pre{font-family:${cssFontStack(t.fonts.mono)}}`,
  ];

  t.headings.levels.slice(0, 4).forEach((h, i) => {
    const tag = `h${i + 1}`;
    const resets = [`tl-h${i + 2}`, `tl-h${i + 3}`, `tl-h${i + 4}`].filter((_, j) => i + 2 + j <= 4);
    if (i === 0 && perChapter) resets.push("tl-figure", "tl-table", "tl-equation");
    rules.push(
      // counter-set, not counter-reset: the counters already exist on .tiptap, and a
      // reset on a sibling would start a nested counter that later headings ignore.
      `${scope} ${tag}{${textRules(h.text, px)};line-height:1.2;margin:${px(h.spaceBeforePt)} 0 ${px(h.spaceAfterPt)};counter-increment:tl-h${i + 1}${resets.length ? `;counter-set:${resets.map((r) => `${r} 0`).join(" ")}` : ""}}`,
    );
    if (i < t.structure.numberedDepth && h.label.trim() !== "") {
      const display = h.layout === "display";
      rules.push(
        `${scope} ${tag}::before{content:${labelContent(h.label, i)};${display ? `display:block;margin-bottom:${px(h.labelGapPt)}` : `margin-right:${px(h.labelGapPt)}`}}`,
      );
    }
  });
  // A level-1 chapter starts a new page in the PDF; show a little more space on screen.
  if (chapters) rules.push(`${scope} h1:not(:first-child){margin-top:${px(t.headings.levels[0]?.spaceBeforePt + 36)}}`);

  // Appendices: the marker restarts top-level numbering, shown as letters.
  const appendixLabel = (i: number) => {
    const expr = Array.from({ length: i + 1 }, (_, j) => (j === 0 ? "counter(tl-h1, upper-alpha)" : `counter(tl-h${j + 1})`)).join(' "." ');
    return expr;
  };
  rules.push(`${scope} .appendix-divider{counter-set:tl-h1 0 tl-h2 0 tl-h3 0 tl-h4 0}`);
  const top = t.headings.levels[0];
  if (top) {
    const label = t.structure.appendixLabel
      .split(/(\{n\})/)
      .filter((p) => p !== "")
      .map((p) => (p === "{n}" ? appendixLabel(0) : q(p)))
      .join(" ");
    rules.push(
      `${scope} .appendix-divider ~ h1::before{content:${label || '""'};${top.layout === "display" ? `display:block;margin-bottom:${px(top.labelGapPt)}` : `margin-right:${px(top.labelGapPt)}`}}`,
    );
  }
  t.headings.levels.slice(1, 4).forEach((h, k) => {
    const i = k + 1;
    if (i < t.structure.numberedDepth && h.label.trim() !== "") {
      const content = h.label
        .split(/(\{n\})/)
        .filter((p) => p !== "")
        .map((p) => (p === "{n}" ? appendixLabel(i) : q(p)))
        .join(" ");
      rules.push(`${scope} .appendix-divider ~ h${i + 1}::before{content:${content}}`);
    }
  });
  if (perChapter) {
    const prefixA = 'counter(tl-h1, upper-alpha) "." ';
    rules.push(
      `${scope} .appendix-divider ~ .node-figure figure.figure figcaption::before{content:${q(t.captions.figureLabel + " ")} ${prefixA}counter(tl-figure) ${q(sep)}}`,
      `${scope} .appendix-divider ~ .table-figure figcaption::before{content:${q(t.captions.tableLabel + " ")} ${prefixA}counter(tl-table) ${q(sep)}}`,
      `${scope} .appendix-divider ~ .node-equation .equation-number::after{content:"(" ${prefixA}counter(tl-equation) ")"}`,
    );
  }

  rules.push(
    `${scope} figure.figure{counter-increment:tl-figure}`,
    `${scope} .table-figure{counter-increment:tl-table}`,
    `${scope} .equation{counter-increment:tl-equation}`,
    `${scope} .equation.unnumbered{counter-increment:none}`,
    `${scope} .equation-number::after{content:"(" ${numberPrefix}counter(tl-equation) ")"}`,
    `${scope} figcaption{font-size:${captionSize}em;text-align:${captionAlign}}`,
    `${scope} figure.figure figcaption::before{content:${q(t.captions.figureLabel + " ")} ${numberPrefix}counter(tl-figure) ${q(sep)};font-weight:${t.captions.labelBold ? 700 : 400}${t.captions.separator === "newline" ? ";display:block" : ""}}`,
    `${scope} .table-figure figcaption::before{content:${q(t.captions.tableLabel + " ")} ${numberPrefix}counter(tl-table) ${q(sep)};font-weight:${t.captions.labelBold ? 700 : 400}${t.captions.separator === "newline" ? ";display:block" : ""}}`,
  );
  return rules.join("\n");
}
