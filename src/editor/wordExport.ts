import type { JSONContent } from "@tiptap/react";
import { convertLatexToMathMl } from "mathlive";
import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import * as api from "../api";
import { isArabicMath } from "./arabicMath";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

/** MathML for every formula in the document, by its LaTeX (as the Word export
 * looks it up), so equations become editable Word equations. A formula MathLive
 * can't convert is left out; the export then shows its LaTeX. */
export function formulasAsMathMl(doc: JSONContent): Record<string, string> {
  const out: Record<string, string> = {};
  const visit = (node: JSONContent) => {
    if (node.type === "equation" || node.type === "inlineMath") {
      const latex = String(node.attrs?.latex ?? "").trim();
      if (latex && !(latex in out)) {
        try {
          out[latex] = `<math xmlns="http://www.w3.org/1998/Math/MathML">${convertLatexToMathMl(latex)}</math>`;
        } catch {
          // Shown as LaTeX in the Word file.
        }
      }
    }
    node.content?.forEach(visit);
  };
  visit(doc);
  return out;
}

/** The key of a formula's picture, as the Word export looks it up
 *  (`nawl_docx::formula_key`): displayed equations are set larger than inline ones. */
const pictureKey = (latex: string, display: boolean) => `${display ? "D" : "I"}:${latex.trim()}`;

/** Pixels per PDF point in the pictures: 288 per inch, sharp in print. */
const PICTURE_SCALE = 4;

/**
 * Pictures of the documents' Arabic formulas, for the Word file: Word can't lay
 * math out right to left, so each goes in as the PDF sets it. The backend sets them
 * one per page; pdf.js draws the pages. Empty when there are none, or when making
 * them fails (the formulas are then written as Word equations, left to right).
 */
export async function arabicFormulaPictures(docs: JSONContent[]): Promise<Record<string, api.FormulaPicture>> {
  const formulas = new Map<string, { latex: string; display: boolean }>();
  const visit = (node: JSONContent) => {
    if (node.type === "equation" || node.type === "inlineMath") {
      const latex = String(node.attrs?.latex ?? "").trim();
      const display = node.type === "equation";
      if (latex && isArabicMath(node.attrs?.style as string | null | undefined)) formulas.set(pictureKey(latex, display), { latex, display });
    }
    node.content?.forEach(visit);
  };
  docs.forEach(visit);
  if (formulas.size === 0) return {};
  const out: Record<string, api.FormulaPicture> = {};
  try {
    const keys = [...formulas.keys()];
    const pages = await api.formulaPictures([...formulas.values()]);
    const task = pdfjs.getDocument({ data: new Uint8Array(pages.pdf) });
    const pdf = await task.promise;
    for (let i = 0; i < Math.min(pdf.numPages, keys.length, pages.sizes.length); i++) {
      const page = await pdf.getPage(i + 1);
      const viewport = page.getViewport({ scale: PICTURE_SCALE });
      const canvas = document.createElement("canvas");
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      // No white behind the formula: the page's own colour shows through.
      await page.render({ canvas, canvasContext: canvas.getContext("2d")!, viewport, background: "rgba(0,0,0,0)" }).promise;
      const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
      if (!blob) continue;
      const [width, height, depth] = pages.sizes[i];
      out[keys[i]] = { png: Array.from(new Uint8Array(await blob.arrayBuffer())), widthPt: width, heightPt: height + depth, depthPt: depth };
    }
    await task.destroy();
  } catch (e) {
    console.warn("formula pictures:", e);
    return {};
  }
  return out;
}
