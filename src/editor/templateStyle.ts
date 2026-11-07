// Turns a template into CSS for the editor page, so the page you type on looks like
// the PDF: margins, fonts, spacing, heading styles with their numbers ("CHAPTER 1",
// "3.1"), and figure/table/equation numbering. The PDF preview stays the exact version.
import { libraryFamily } from "./libraryFonts";
import type { Template } from "../api";
import type { FontChoice } from "../bindings/FontChoice";
import type { TextStyle } from "../bindings/TextStyle";
import { ABJAD_COUNTER_STYLE, ARABIC_FALLBACKS, docLanguage } from "./language";

/** Width of the on-screen page in CSS pixels. */
export const PAGE_WIDTH_PX = 800;
const MM_PER_PT = 25.4 / 72.27;

export function cssFontStack(f: FontChoice): string {
  if (f.kind === "system") return `"${f.family.replace(/"/g, "")}", serif`;
  // Loaded from the app's font library (libraryFonts.ts).
  if (f.kind === "library") return `"${libraryFamily(f.id)}", "Amiri", serif`;
  return {
    latinModern: '"Latin Modern Roman", "CMU Serif", serif',
    termes: '"TeX Gyre Termes", "Times New Roman", Times, serif',
    pagella: '"TeX Gyre Pagella", Palatino, "Palatino Linotype", serif',
    bonum: '"TeX Gyre Bonum", "Bookman Old Style", serif',
    schola: '"TeX Gyre Schola", "Century Schoolbook", serif',
    heros: '"TeX Gyre Heros", Helvetica, Arial, sans-serif',
    adventor: '"TeX Gyre Adventor", "Avant Garde", sans-serif',
    cursor: '"TeX Gyre Cursor", "Courier New", monospace',
    amiri: '"Amiri", "Traditional Arabic", serif',
  }[f.font];
}

/** The page's font: the template's main font for Latin letters, then its Arabic font
 *  and common Arabic fonts for Arabic letters (each letter takes the first font that
 *  has it), as in the PDF. */
export function pageFontStack(t: Template, latin = cssFontStack(t.fonts.main)): string {
  const parts = latin.split(",").map((s) => s.trim());
  const generic = parts.pop() ?? "serif";
  const a = t.fonts.arabic;
  const arabic = a?.kind === "system" ? [`"${a.family.replace(/"/g, "")}"`] : a?.kind === "library" ? [`"${libraryFamily(a.id)}"`] : [];
  return [...parts, ...arabic, ...ARABIC_FALLBACKS, generic].join(", ");
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

// ---------------------------------------------------------------- presentations

/** Beamer themes Texloom offers, roughly by look. */
export const BEAMER_THEMES = [
  "default", "Madrid", "metropolis", "CambridgeUS", "Boadilla", "AnnArbor", "Antibes", "Bergen", "Berkeley", "Berlin",
  "Copenhagen", "Darmstadt", "Dresden", "Frankfurt", "Goettingen", "Hannover", "Ilmenau", "JuanLesPins", "Luebeck",
  "Malmoe", "Marburg", "Montpellier", "PaloAlto", "Pittsburgh", "Rochester", "Singapore", "Szeged", "Warsaw",
];
export const BEAMER_COLOR_THEMES = [
  "albatross", "beaver", "beetle", "crane", "default", "dolphin", "dove", "fly", "lily", "orchid", "rose", "seagull",
  "seahorse", "spruce", "whale", "wolverine",
];
export const BEAMER_FONT_THEMES = ["default", "professionalfonts", "serif", "structurebold", "structureitalicserif", "structuresmallcapsserif"];

/** Colours that approximate a beamer theme on screen (the PDF is exact). */
export function slideColors(
  theme: string,
  colorTheme: string,
  accent?: string,
): { bar: string; barFg: string; structure: string; alert: string; example: string } {
  const blue = "#3333b3";
  const base: Record<string, { bar: string; barFg: string; structure: string }> = {
    default: { bar: "transparent", barFg: blue, structure: blue },
    metropolis: { bar: "#23373b", barFg: "#ffffff", structure: "#23373b" },
    CambridgeUS: { bar: "transparent", barFg: "#a31f34", structure: "#a31f34" },
    Boadilla: { bar: "transparent", barFg: blue, structure: blue },
    Pittsburgh: { bar: "transparent", barFg: blue, structure: blue },
    Bergen: { bar: "transparent", barFg: blue, structure: blue },
    Montpellier: { bar: "transparent", barFg: blue, structure: blue },
    Rochester: { bar: blue, barFg: "#ffffff", structure: blue },
    AnnArbor: { bar: "#002d72", barFg: "#ffcb05", structure: "#002d72" },
  };
  const c = { ...(base[theme] ?? { bar: blue, barFg: "#ffffff", structure: blue }), alert: "#cc0000", example: "#007a00" };
  const tint: Record<string, string> = {
    beaver: "#a60000", crane: "#c77a00", dolphin: "#2d4a8a", seahorse: "#5e5ea8", whale: "#262680", albatross: "#2e3a7a",
    fly: "#4a4a4a", seagull: "#5a5a5a", spruce: "#21544b", wolverine: "#c08a00", beetle: "#333366", dove: "#555555",
  };
  const t = accent ?? tint[colorTheme];
  if (t) {
    c.structure = t;
    if (c.bar !== "transparent") c.bar = t;
    else c.barFg = t;
  }
  return c;
}

/** Slide size on screen: the page width, and the height for the aspect ratio. */
export function slideSizePx(t: Template): { width: number; height: number } {
  const ratio = { "16:9": 9 / 16, "16:10": 10 / 16, "4:3": 3 / 4 }[t.presentation?.aspectRatio ?? "16:9"];
  return { width: PAGE_WIDTH_PX, height: Math.round(PAGE_WIDTH_PX * ratio) };
}

/** CSS for the slide you type on: the slide's size, the theme's colours and fonts. */
/** What a presentation's footer shows on screen, from the document. */
export interface DeckInfo {
  title: string;
  author: string;
  date: string;
  institute: string;
  /** Total number of frames in the PDF (roughly: title, outline, slides, references). */
  pages: number;
  /** Frames before the first slide (title, outline): where slide numbers start. */
  firstNumber: number;
  /** The logo is set and shown on every slide. */
  logo: boolean;
}

/** Themes whose footer is beamer's "infolines": presenter | title | date and number. */
const INFOLINES = ["Madrid", "AnnArbor", "CambridgeUS", "Boadilla", "Pittsburgh", "Szeged", "Malmoe", "Dresden"];

/** A footer part as a CSS `content` value: text plus the slide-number counter. */
function footerContent(text: string, info: DeckInfo): string {
  const parts = text.split(/(\{[a-z]+\})/).filter((x) => x !== "");
  const values: Record<string, string> = {
    "{author}": q(info.author),
    "{title}": q(info.title),
    "{date}": q(info.date),
    "{institute}": q(info.institute),
    "{section}": '""',
    "{page}": "counter(tl-slide)",
    "{pages}": q(String(info.pages)),
  };
  return parts.map((part) => values[part] ?? q(part)).join(" ") || '""';
}

function presentationCss(t: Template, scope: string, info?: DeckInfo): string {
  const p = t.presentation;
  const { width, height } = slideSizePx(t);
  const slideWidthMm = p?.aspectRatio === "4:3" ? 128 : 160;
  const k = width / slideWidthMm; // px per mm
  const px = (pt: number) => `${(pt * MM_PER_PT * k).toFixed(2)}px`;
  const mm = (v: number) => `${(v * k).toFixed(1)}px`;
  const size = p?.fontSizePt ?? 11;
  const c = slideColors(p?.theme ?? "Madrid", p?.colorTheme ?? "", p?.accentColor);
  const serif = p?.fontTheme?.includes("serif") && !p.fontTheme.startsWith("structure");
  // Footer line and logo, at the slide's real bottom edge.
  const footH = Math.round(height * 0.045);
  const slideChrome = (): string[] => {
    const rules = [
      `${scope} .tiptap{counter-reset:tl-slide ${info?.firstNumber ?? 0}}`,
      `${scope} .slide{counter-increment:tl-slide}`,
      `${scope} .slide-body{padding-bottom:${footH}px}`,
      `${scope} .slide-footer{position:absolute;left:0;right:0;top:${height - footH}px;height:${footH}px;display:none;grid-template-columns:1fr 1fr 1fr;align-items:center;font-size:${px(size * 0.55)};pointer-events:none;user-select:none}`,
      `${scope} .slide-footer > span{padding:0 ${mm(3)};white-space:nowrap;overflow:hidden;text-overflow:ellipsis;line-height:${footH}px}`,
      `${scope} .slide-foot-center{text-align:center}`,
      `${scope} .slide-foot-right{text-align:right}`,
      `${scope} .slide-logo{position:absolute;right:${mm(3)};top:${Math.round(height - footH - 9.5 * k)}px;width:${mm(24)};height:${mm(8)};background:var(--slide-logo, none) right bottom / contain no-repeat;pointer-events:none;display:${info?.logo ? "block" : "none"}}`,
      `${scope} .slide[data-plain] .slide-footer, ${scope} .slide[data-plain] .slide-logo{display:none}`,
    ];
    if (!info) return rules;
    const style = p?.footer.style ?? "theme";
    let parts: [string, string, string] | null = null;
    if (style === "custom") parts = [p!.footer.left, p!.footer.center, p!.footer.right];
    else if (style === "theme" && INFOLINES.includes(p?.theme ?? "")) parts = ["{author}", "{title}", "{date}    {page} / {pages}"];
    if (parts) {
      const solid = c.bar !== "transparent";
      rules.push(
        `${scope} .slide-footer{display:grid;${solid ? `background:${c.structure};color:#fff` : `color:${c.structure};border-top:1px solid color-mix(in srgb, ${c.structure} 30%, transparent)`}}`,
        `${scope} .slide-foot-left::before{content:${footerContent(parts[0], info)}}`,
        `${scope} .slide-foot-center::before{content:${footerContent(parts[1], info)}}`,
        `${scope} .slide-foot-right::before{content:${footerContent(parts[2], info)}}`,
      );
      if (solid) rules.push(`${scope} .slide-foot-center{background:color-mix(in srgb, ${c.structure} 75%, #fff)}`, `${scope} .slide-foot-right{background:color-mix(in srgb, ${c.structure} 55%, #fff);color:${c.structure}}`);
    }
    return rules;
  };
  const font = pageFontStack(t, serif ? '"Latin Modern Roman", "CMU Serif", serif' : '"Latin Modern Sans", "CMU Sans Serif", "Helvetica Neue", Arial, sans-serif');
  return [
    `${scope}{width:${width}px;max-width:none;padding:0;font-family:${font};font-size:${px(size)};line-height:1.3}`,
    // Each slide is a page-coloured card at the slide's size; cards stack down the page.
    `${scope} .slide{display:flow-root;position:relative;box-sizing:border-box;min-height:${height}px;margin:0 0 ${mm(5)};background-color:var(--page-bg, #f4efe6);color:var(--page-fg, #1d2228);padding-bottom:${mm(6)};border-radius:2px}`,
    // Content past the slide's bottom edge sits on a red tint: it won't fit in the PDF.
    // An overlay from the bottom edge down (a gradient stop would ignore the page zoom).
    `${scope} .slide::after{content:"";position:absolute;left:0;right:0;top:${height}px;bottom:0;background:rgba(220, 38, 38, 0.12);pointer-events:none}`,
    `${scope} .slide-title{background:${c.bar};color:${c.barFg};font-size:${px(size * 1.2)};line-height:1.25;padding:${mm(2.5)} ${mm(4)};margin-bottom:${mm(4)};min-height:1.25em;border-radius:2px 2px 0 0}`,
    `${scope} .slide-title:has(> br.ProseMirror-trailingBreak:only-child)::before{content:"Slide title";float:inline-start;height:0;opacity:0.55;pointer-events:none}`,
    `${scope} .slide-body > :not(.slide-title){margin-left:${mm(5)};margin-right:${mm(5)}}`,
    ...slideChrome(),
    `${scope} .slide p{margin:0 0 ${mm(1.5)};text-indent:0}`,
    `${scope} .slide ul, ${scope} .slide ol{margin:${mm(1)} 0 ${mm(2)};padding-inline-start:${mm(6)}}`,
    `${scope} .slide li::marker{color:${c.structure}}`,
    `${scope} .slide :is(h1,h2,h3,h4){font-size:1em;font-weight:700;color:${c.structure};margin:${mm(2)} 0 ${mm(1)}}`,
    `${scope} :is(h1,h2,h3,h4)::before{content:none}`,
    // Section headings between slides: a label in the app's own style, like a divider.
    `${scope} .tiptap > :is(h1,h2,h3,h4){font:600 21px/1.3 system-ui, sans-serif;color:var(--text, #1d2228);margin:${mm(7)} 0 ${mm(3)};padding:0 0 4px;border-bottom:1px solid var(--border, #ccc);text-align:start}`,
    `${scope} .tiptap > :first-child:is(h1,h2,h3,h4){margin-top:0}`,
    `${scope} .tiptap > :is(h1,h2,h3,h4):has(> br.ProseMirror-trailingBreak:only-child)::before{content:"Section title";float:inline-start;height:0;opacity:0.5}`,
    // Anything outside a slide (it gets a slide of its own in the PDF).
    `${scope} .tiptap > :not(.slide):not(h1,h2,h3,h4){background:var(--page-bg, #f4efe6);color:var(--page-fg, #1d2228);margin:0 0 ${mm(5)};padding:${mm(2)} ${mm(5)};border-radius:2px;opacity:0.9}`,
    // The empty line the editor keeps at the end: just room for the cursor.
    `${scope} .tiptap > p:last-child:has(> br.ProseMirror-trailingBreak:only-child){background:transparent;padding:0;min-height:1em}`,
    `${scope} .slide-columns{display:flex;gap:${mm(4)};align-items:flex-start}`,
    `${scope} .slide-column{min-width:0}`,
    `${scope} .slide-box{margin:${mm(2)} 0;border-radius:4px;overflow:hidden;background:color-mix(in srgb, ${c.structure} 8%, var(--page-bg, #fff))}`,
    `${scope} .slide-box-title{display:flex;gap:6px;align-items:center;background:${c.structure};color:#fff;padding:${mm(1)} ${mm(2)}}`,
    `${scope} .slide-box-alert{background:color-mix(in srgb, ${c.alert} 8%, var(--page-bg, #fff))}`,
    `${scope} .slide-box-alert .slide-box-title{background:${c.alert}}`,
    `${scope} .slide-box-example{background:color-mix(in srgb, ${c.example} 8%, var(--page-bg, #fff))}`,
    `${scope} .slide-box-example .slide-box-title{background:${c.example}}`,
    `${scope} .slide-box-body{padding:${mm(1.5)} ${mm(2)}}`,
    `${scope} figure.figure{margin:${mm(1)} 0}`,
    `${scope} figcaption{font-size:0.8em;text-align:center}`,
    // No numbers on screen: hidden slides don't count in CSS counters (the PDF numbers them).
    `${scope} figure.figure figcaption::before{content:"Figure: ";color:${c.structure}}`,
    `${scope} .table-figure figcaption::before{content:"Table: ";color:${c.structure}}`,
  ].join("\n");
}

/** CSS rules scoped to `scope` (the editor page element). */
export function templateCss(t: Template, scope = ".page", deck?: DeckInfo): string {
  const css = t.kind === "presentation" ? presentationCss(t, scope, deck) : articleCss(t, scope);
  // Arabic-Indic digits (٣٫١) for every number the page counts, as in the PDF.
  return docLanguage(t).digits === "arabicIndic" ? css.replace(/counter\((tl-[\w-]+)\)/g, "counter($1, arabic-indic)") : css;
}

function articleCss(t: Template, scope: string): string {
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
    // The binding (inner) margin is on the side text starts from: the right in Arabic.
    `${scope}{width:${PAGE_WIDTH_PX}px;max-width:none;padding-block:${mm(t.page.margins.topMm)} ${mm(t.page.margins.bottomMm)};padding-inline:${mm(inner)} ${mm(t.page.margins.outerMm)};font-family:${pageFontStack(t)};font-size:${px(t.fonts.baseSizePt)};line-height:${lineHeight}}`,
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
      // A publisher or standard class sets its own headings (the designer hides the
      // Headings panel), and they start where the line starts: the right in Arabic.
      `${scope} ${tag}{${textRules(t.base ? { ...h.text, align: "start" as TextStyle["align"] } : h.text, px)};line-height:1.2;margin:${px(h.spaceBeforePt)} 0 ${px(h.spaceAfterPt)};counter-increment:tl-h${i + 1}${resets.length ? `;counter-set:${resets.map((r) => `${r} 0`).join(" ")}` : ""}}`,
    );
    if (i < t.structure.numberedDepth && h.label.trim() !== "") {
      const display = h.layout === "display";
      rules.push(
        `${scope} ${tag}::before{content:${labelContent(h.label, i)};${display ? `display:block;margin-bottom:${px(h.labelGapPt)}` : `margin-inline-end:${px(h.labelGapPt)}`}}`,
      );
      // In an empty heading (just ProseMirror's trailing <br>) WebKit draws the caret
      // at the start of the line, left of the number. Floated, the number narrows the
      // line box, so the caret starts after it.
      if (!display) rules.push(`${scope} ${tag}:has(> br.ProseMirror-trailingBreak:only-child)::before{float:inline-start}`);
    }
  });
  // A level-1 chapter starts a new page in the PDF; show a little more space on screen.
  if (chapters) rules.push(`${scope} h1:not(:first-child){margin-top:${px(t.headings.levels[0]?.spaceBeforePt + 36)}}`);

  // Appendices: the marker restarts top-level numbering, shown as letters.
  // Abjad letters in Arabic (ا ب ج), as polyglossia prints \Alph.
  const letters = docLanguage(t).main === "arabic" ? "tl-abjad" : "upper-alpha";
  const appendixLabel = (i: number) => {
    const expr = Array.from({ length: i + 1 }, (_, j) => (j === 0 ? `counter(tl-h1, ${letters})` : `counter(tl-h${j + 1})`)).join(' "." ');
    return expr;
  };
  // Numbered lists as LaTeX's enumerate: 1. (a) i. A.; abjad letters in Arabic.
  rules.push(
    `@counter-style tl-abjad{${ABJAD_COUNTER_STYLE}}`,
    `@counter-style tl-sublist{system:extends ${docLanguage(t).main === "arabic" ? "tl-abjad" : "lower-alpha"};prefix:"(";suffix:") "}`,
    `${scope} ol ol{list-style-type:tl-sublist}`,
    `${scope} ol ol ol{list-style-type:lower-roman}`,
    `${scope} ol ol ol ol{list-style-type:upper-alpha}`,
  );
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
    const prefixA = `counter(tl-h1, ${letters}) "." `;
    rules.push(
      `${scope} .appendix-divider ~ .node-figure figure.figure figcaption::before{content:${q(t.captions.figureLabel + " ")} ${prefixA}counter(tl-figure) ${q(sep)}}`,
      `${scope} .appendix-divider ~ .table-figure figcaption::before{content:${q(t.captions.tableLabel + " ")} ${prefixA}counter(tl-table) ${q(sep)}}`,
      `${scope} .appendix-divider ~ .node-equation .equation-number::after{content:"(" ${prefixA}counter(tl-equation) ")"}`,
    );
  }

  rules.push(
    // Unnumbered headings: no label, and the counters skip them (like \chapter*).
    `${scope} :is(h1,h2,h3,h4)[data-unnumbered]{counter-increment:none;counter-set:none}`,
    `${scope} :is(h1,h2,h3,h4)[data-unnumbered]::before{content:none !important}`,
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
