// Help while typing a formula in an Arabic document: MathLive's field edits left to
// right, so a live preview shows the formula as it will be set (right to left), with
// a palette of Arabic functions and symbols, and a switch to write this one formula
// the other way (an international formula in an Arabic book, or the reverse).
import type { MathfieldElement } from "mathlive";
import type { Editor } from "@tiptap/react";
import { translate, uiLanguage } from "../i18n";
import { isArabicMath, mathSettings, toArabicNotation, toLatinNotation } from "./arabicMath";
import { staticMarkup } from "./InlineMath";

/** What a palette button inserts (`#?` is a place to fill in), and how it's shown;
 *  `mirrored` signs are drawn the way the formula will show them. */
const PALETTE: { label: string; insert: string; mirrored?: boolean }[] = [
  { label: "جا", insert: "جا" },
  { label: "جتا", insert: "جتا" },
  { label: "ظا", insert: "ظا" },
  { label: "لو", insert: "لو" },
  { label: "لط", insert: "لط" },
  { label: "نهـا", insert: "نهـا_{#?}" },
  { label: "مجـ", insert: "مجـ_{#?}^{#?}" },
  { label: "∑", insert: "\\sum_{#?}^{#?}", mirrored: true },
  { label: "∫", insert: "\\int_{#?}^{#?}", mirrored: true },
  { label: "√", insert: "\\sqrt{#?}", mirrored: true },
  { label: "▫⁄▫", insert: "\\frac{#?}{#?}" },
  { label: "▫²", insert: "^{#?}", mirrored: true },
  { label: "▫₂", insert: "_{#?}", mirrored: true },
  { label: "≤", insert: "\\le", mirrored: true },
  { label: "≥", insert: "\\ge", mirrored: true },
  { label: "≠", insert: "\\ne" },
  { label: "←", insert: "\\to" },
  { label: "∞", insert: "\\infty" },
  { label: "×", insert: "\\times" },
  { label: "÷", insert: "\\div" },
  { label: "π", insert: "\\pi" },
];

/** The formulas whose letters change, and their new LaTeX: Latin variables into
 *  Arabic letters (x → س) in the formulas written the Arabic way, or Arabic letters
 *  back into Latin ones (س → x) in all of them. `except`: formulas with this style
 *  of their own are left alone, and the others are converted whatever the template
 *  says (the template's way of writing formulas is just being changed). */
function lettersToConvert(editor: Editor, to: "arabic" | "latin", except?: "arabic" | "international") {
  const changes: { pos: number; attrs: Record<string, unknown> }[] = [];
  editor.state.doc.descendants((node, pos) => {
    if (node.type.name !== "inlineMath" && node.type.name !== "equation") return true;
    const latex = (node.attrs.latex as string) ?? "";
    const wanted = except ? node.attrs.style !== except : to === "latin" || isArabicMath(node.attrs.style);
    const next = !wanted ? latex : to === "latin" ? toLatinNotation(latex) : toArabicNotation(latex);
    if (next !== latex) changes.push({ pos, attrs: { ...node.attrs, latex: next } });
    return false;
  });
  return changes;
}

/** How many formulas `convertAllFormulas` would change. */
export const formulasToConvert = (editor: Editor, to: "arabic" | "latin", except?: "arabic" | "international") => lettersToConvert(editor, to, except).length;

/** Changes the letters of the document's formulas (see `lettersToConvert`) in one
 *  step that Undo takes back. */
export function convertAllFormulas(editor: Editor, to: "arabic" | "latin", except?: "arabic" | "international") {
  const changes = lettersToConvert(editor, to, except);
  if (!changes.length) return;
  const tr = editor.state.tr;
  for (const c of changes) tr.setNodeMarkup(c.pos, undefined, c.attrs);
  editor.view.dispatch(tr);
}

export interface MathAssist {
  el: HTMLElement;
  /** The formula or its style changed: draw the preview again. */
  update: () => void;
}

/**
 * The helper for a formula being edited in `field`; null outside Arabic documents.
 * `display`: an equation of its own. `setStyle` stores the formula's own way of
 * writing (null: the template's).
 */
export function createMathAssist(opts: {
  field: MathfieldElement;
  display: boolean;
  getLatex: () => string;
  getStyle: () => string | null | undefined;
  setStyle: (style: string | null) => void;
  /** Stores a new formula (the field follows). */
  setLatex: (latex: string) => void;
  editor: Editor;
}): MathAssist | null {
  if (!mathSettings().arabicDocument) return null;
  const t = (key: Parameters<typeof translate>[1]) => translate(uiLanguage(), key);
  const el = document.createElement("div");
  el.className = `math-assist${opts.display ? " display" : ""}`;
  el.contentEditable = "false";
  el.dir = uiLanguage() === "ar" ? "rtl" : "ltr";
  // Clicks here mustn't take the focus from the field (it would close).
  el.addEventListener("mousedown", (e) => e.preventDefault());

  const head = document.createElement("div");
  head.className = "math-assist-head";
  const styles = (["arabic", "international"] as const).map((style) => {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = t(style === "arabic" ? "editor.mathArabic" : "editor.mathInternational");
    b.title = t(style === "arabic" ? "editor.mathArabicTitle" : "editor.mathInternationalTitle");
    b.addEventListener("click", () => {
      // The template's own way: no setting on the formula.
      opts.setStyle(style === (mathSettings().arabic ? "arabic" : "international") ? null : style);
    });
    head.appendChild(b);
    return { style, b };
  });

  // The letters changed between the two notations (x → س, or back: س → x), in this
  // formula or all of the document's. Each pair shows when it has something to do.
  const spacer = document.createElement("span");
  spacer.className = "math-assist-spacer";
  head.appendChild(spacer);
  const converters = (["arabic", "latin"] as const).map((to) => {
    const one = document.createElement("button");
    one.type = "button";
    one.className = "math-assist-convert";
    one.textContent = t(to === "arabic" ? "editor.mathConvert" : "editor.mathConvertBack");
    one.title = t(to === "arabic" ? "editor.mathConvertTitle" : "editor.mathConvertBackTitle");
    one.addEventListener("click", () => opts.setLatex(to === "arabic" ? toArabicNotation(opts.getLatex()) : toLatinNotation(opts.getLatex())));
    const all = document.createElement("button");
    all.type = "button";
    all.className = "math-assist-convert";
    all.textContent = t("editor.mathConvertAll");
    all.title = t(to === "arabic" ? "editor.mathConvertAllTitle" : "editor.mathConvertBackAllTitle");
    all.addEventListener("click", () => convertAllFormulas(opts.editor, to));
    head.append(one, all);
    return { to, one, all };
  });

  const preview = document.createElement("div");
  preview.className = "math-assist-preview";
  preview.dir = "ltr";

  const palette = document.createElement("div");
  palette.className = "math-assist-palette";
  // Arabic letters and signs read right to left, whatever the interface language.
  palette.dir = "rtl";
  for (const item of PALETTE) {
    const b = document.createElement("button");
    b.type = "button";
    const label = document.createElement("span");
    label.textContent = item.label;
    if (item.mirrored) label.className = "mirrored";
    b.appendChild(label);
    b.addEventListener("click", () => {
      opts.field.insert(item.insert, { focus: true, selectionMode: "placeholder" });
      opts.field.dispatchEvent(new Event("input", { bubbles: true }));
    });
    palette.appendChild(b);
  }
  el.append(head, preview, palette);

  const update = () => {
    const arabic = isArabicMath(opts.getStyle());
    for (const { style, b } of styles) b.classList.toggle("active", (style === "arabic") === arabic);
    el.classList.toggle("arabic", arabic);
    const latex = opts.getLatex();
    // Into Arabic letters: a formula written the Arabic way that has Latin ones. Back
    // into Latin letters: any formula that has Arabic ones.
    for (const { to, one, all } of converters) {
      const useful = to === "arabic" ? arabic && toArabicNotation(latex) !== latex : toLatinNotation(latex) !== latex;
      one.hidden = !useful;
      all.hidden = !useful;
    }
    preview.innerHTML = arabic && latex.trim() ? staticMarkup(latex, opts.display, opts.getStyle()) : "";
  };
  update();
  return { el, update };
}
