import type { JSONContent } from "@tiptap/react";
import { convertLatexToMathMl } from "mathlive";

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
