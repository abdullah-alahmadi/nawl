// Help while typing a formula in an Arabic document: MathLive's field edits left to
// right, so a live preview shows the formula as it will be set (right to left), with
// a palette of Arabic functions and symbols, and a switch to write this one formula
// the other way (an international formula in an Arabic book, or the reverse).
import type { MathfieldElement } from "mathlive";
import type { Editor } from "@tiptap/react";
import { translate, uiLanguage } from "../i18n";
import { isArabicMath, mathSettings, toArabicNotation } from "./arabicMath";
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

/** Every Arabic-style formula of the document with its Latin variables as Arabic
 *  letters (x → س), in one step that Undo takes back. */
export function convertAllFormulas(editor: Editor) {
  const tr = editor.state.tr;
  editor.state.doc.descendants((node, pos) => {
    if (node.type.name !== "inlineMath" && node.type.name !== "equation") return true;
    const latex = (node.attrs.latex as string) ?? "";
    const next = isArabicMath(node.attrs.style) ? toArabicNotation(latex) : latex;
    if (next !== latex) tr.setNodeMarkup(pos, undefined, { ...node.attrs, latex: next });
    return false;
  });
  if (tr.docChanged) editor.view.dispatch(tr);
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

  // Latin letters into Arabic ones (x → س): this formula, or all of the document's.
  const spacer = document.createElement("span");
  spacer.className = "math-assist-spacer";
  const convert = document.createElement("button");
  convert.type = "button";
  convert.className = "math-assist-convert";
  convert.textContent = t("editor.mathConvert");
  convert.title = t("editor.mathConvertTitle");
  convert.addEventListener("click", () => opts.setLatex(toArabicNotation(opts.getLatex())));
  const convertAll = document.createElement("button");
  convertAll.type = "button";
  convertAll.className = "math-assist-convert";
  convertAll.textContent = t("editor.mathConvertAll");
  convertAll.title = t("editor.mathConvertAllTitle");
  convertAll.addEventListener("click", () => convertAllFormulas(opts.editor));
  head.append(spacer, convert, convertAll);

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
    convert.disabled = toArabicNotation(latex) === latex;
    preview.innerHTML = arabic && latex.trim() ? staticMarkup(latex, opts.display, opts.getStyle()) : "";
  };
  update();
  return { el, update };
}
