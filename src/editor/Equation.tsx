// Display equation node: a MathLive field embedded in the TipTap document.
// The node stores plain LaTeX in its `latex` attribute; MathLive edits it visually.
import { useEffect, useRef } from "react";
import { MathfieldElement } from "mathlive";
import {
  Node,
  mergeAttributes,
  nodeInputRule,
  NodeViewWrapper,
  ReactNodeViewRenderer,
  type ReactNodeViewProps,
} from "@tiptap/react";

MathfieldElement.fontsDirectory = "/mathlive/fonts";
MathfieldElement.soundsDirectory = null;

declare module "@tiptap/react" {
  interface Commands<ReturnType> {
    equation: { insertEquation: (latex?: string) => ReturnType };
  }
}

const ALIGNED = /^\s*\\begin\{aligned\}([\s\S]*)\\end\{aligned\}\s*$/;

/** Multi-line equations are an `aligned` block: rows split by \\, aligned at &. */
export function isMultiLine(latex: string) {
  return ALIGNED.test(latex);
}

function toMultiLine(latex: string) {
  return isMultiLine(latex) ? latex : `\\begin{aligned}${latex}\\end{aligned}`;
}

/** Back to one line: rows joined with a wide space, alignment marks dropped. */
function toSingleLine(latex: string) {
  const m = latex.match(ALIGNED);
  if (!m) return latex;
  return m[1]
    .split("\\\\")
    .map((row) => row.replace(/&/g, "").replace(/\\placeholder\{\}/g, "").trim())
    .filter(Boolean)
    .join("\\quad ");
}

function EquationView({ node, updateAttributes, selected, editor, getPos }: ReactNodeViewProps) {
  const host = useRef<HTMLDivElement>(null);
  const field = useRef<MathfieldElement | null>(null);

  // Create the math field once; React doesn't manage its internals.
  useEffect(() => {
    const mf = new MathfieldElement();
    mf.mathVirtualKeyboardPolicy = "manual";
    mf.value = node.attrs.latex ?? "";
    mf.addEventListener("input", () => updateAttributes({ latex: mf.value }));
    // Leaving the field with arrows/Escape returns the cursor to the text around it.
    mf.addEventListener("move-out", (ev) => {
      const pos = typeof getPos === "function" ? getPos() : undefined;
      if (pos === undefined) return;
      const after = (ev as CustomEvent<{ direction: string }>).detail.direction !== "backward" &&
        (ev as CustomEvent<{ direction: string }>).detail.direction !== "upward";
      editor.chain().focus().setTextSelection(after ? pos + node.nodeSize : pos).run();
    });
    host.current?.appendChild(mf);
    field.current = mf;
    if (!mf.value) requestAnimationFrame(() => mf.focus());
    return () => mf.remove();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Keep the field in sync when the attribute changes from outside (undo, open file).
  useEffect(() => {
    const mf = field.current;
    if (mf && mf.value !== node.attrs.latex) mf.value = node.attrs.latex ?? "";
  }, [node.attrs.latex]);

  const numbered = node.attrs.numbered !== false;
  const multi = isMultiLine(node.attrs.latex ?? "");
  const setLatex = (latex: string) => {
    updateAttributes({ latex });
    const mf = field.current;
    if (mf) {
      mf.value = latex;
      requestAnimationFrame(() => mf.focus());
    }
  };
  const addLine = () => {
    const mf = field.current;
    if (!mf) return;
    if (!isMultiLine(mf.value)) mf.value = toMultiLine(mf.value);
    mf.focus();
    // The cursor has to be inside the aligned block (in its last row): at the very
    // end, MathLive would wrap everything in \displaylines instead of adding a row.
    mf.executeCommand("moveToMathfieldEnd");
    mf.executeCommand("moveToPreviousChar");
    mf.executeCommand("addRowAfter");
    updateAttributes({ latex: mf.value });
  };

  return (
    <NodeViewWrapper
      className={`equation${selected ? " is-selected" : ""}${numbered ? "" : " unnumbered"}`}
      data-equation=""
    >
      <div ref={host} className="equation-field" />
      {/* The number "(3.1)" comes from CSS counters (see templateStyle.ts). */}
      {numbered && <span className="equation-number" aria-hidden />}
      <div className="equation-tools" contentEditable={false}>
        <label title="Numbered equations can be referenced">
          <input type="checkbox" checked={numbered} onChange={(e) => updateAttributes({ numbered: e.target.checked })} />
          Numbered
        </label>
        <label title="Several lines, aligned at & (e.g. at the = signs)">
          <input
            type="checkbox"
            checked={multi}
            onChange={(e) => setLatex(e.target.checked ? toMultiLine(node.attrs.latex ?? "") : toSingleLine(node.attrs.latex ?? ""))}
          />
          Multi-line
        </label>
        <button type="button" onClick={addLine} title="Add a line below (type & where lines should align)">
          + Line
        </button>
      </div>
    </NodeViewWrapper>
  );
}

export const Equation = Node.create({
  name: "equation",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,

  addAttributes() {
    return { latex: { default: "" }, numbered: { default: true } };
  },

  parseHTML() {
    return [
      {
        tag: "div[data-equation]",
        getAttrs: (el) => ({
          latex: (el as HTMLElement).dataset.latex ?? "",
          numbered: (el as HTMLElement).dataset.numbered !== "false",
        }),
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      "div",
      mergeAttributes(HTMLAttributes, {
        "data-equation": "",
        "data-latex": node.attrs.latex,
        "data-numbered": String(node.attrs.numbered !== false),
      }),
    ];
  },

  addNodeView() {
    // stopEvent: keystrokes inside MathLive must not reach ProseMirror.
    return ReactNodeViewRenderer(EquationView, { stopEvent: () => true });
  },

  addCommands() {
    return {
      insertEquation:
        (latex = "") =>
        ({ commands }) =>
          commands.insertContent({ type: this.name, attrs: { latex } }),
    };
  },

  addInputRules() {
    // Typing "$$ " at the start of an empty paragraph inserts an equation.
    return [nodeInputRule({ find: /^\$\$\s$/, type: this.type })];
  },
});
