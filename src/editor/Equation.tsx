// Display equation node: a MathLive field embedded in the TipTap document.
// The node stores plain LaTeX in its `latex` attribute; MathLive edits it visually.
import { useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { MathfieldElement } from "mathlive";
import { staticMarkup } from "./InlineMath";
import { mathSettings, onMathSettings } from "./arabicMath";
import { createMathAssist, type MathAssist } from "./mathAssist";
import {
  Node,
  mergeAttributes,
  nodeInputRule,
  NodeViewWrapper,
  ReactNodeViewRenderer,
  type ReactNodeViewProps,
} from "@tiptap/react";
import { useT } from "../i18n";

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
  const t = useT();
  const host = useRef<HTMLDivElement>(null);
  const field = useRef<MathfieldElement | null>(null);
  // A live MathLive field only while the equation is being edited (a new, empty one
  // starts so); otherwise static markup. Live fields for every equation of a long
  // book made opening it slow.
  const [editing, setEditing] = useState(!node.attrs.latex);
  // Something to do once the field exists ("add line" clicked on a static equation).
  const pending = useRef<((mf: MathfieldElement) => void) | null>(null);
  const assist = useRef<MathAssist | null>(null);
  const styleRef = useRef<string | null>(node.attrs.style ?? null);
  styleRef.current = node.attrs.style ?? null;
  // Drawn again when the template's way of writing formulas changes.
  const math = useSyncExternalStore(onMathSettings, mathSettings);
  const markup = useMemo(() => staticMarkup(node.attrs.latex ?? "", true, node.attrs.style), [node.attrs.latex, node.attrs.style, math]);

  // Create the math field while editing; React doesn't manage its internals.
  useEffect(() => {
    if (!editing) return;
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
    // Back to static markup once the field loses the focus, or the text cursor goes
    // elsewhere in the document (should a blur be missed).
    const done = () => setEditing(false);
    mf.addEventListener("focusout", done);
    mf.addEventListener("blur", done);
    const elsewhere = () => {
      if (document.activeElement !== mf) done();
    };
    editor.on("selectionUpdate", elsewhere);
    host.current?.appendChild(mf);
    field.current = mf;
    // The right-to-left preview and the palette (Arabic documents).
    assist.current = createMathAssist({
      field: mf,
      display: true,
      getLatex: () => mf.value,
      getStyle: () => styleRef.current,
      setStyle: (style) => updateAttributes({ style }),
      setLatex: (latex) => updateAttributes({ latex }),
      editor,
    });
    if (assist.current) host.current?.appendChild(assist.current.el);
    // Once attached (a timer, not an animation frame: hidden windows run no frames).
    window.setTimeout(() => {
      mf.focus();
      pending.current?.(mf);
      pending.current = null;
    });
    return () => {
      editor.off("selectionUpdate", elsewhere);
      mf.remove();
      assist.current?.el.remove();
      assist.current = null;
      field.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editing]);

  // Keep the field in sync when the attribute changes from outside (undo, open file).
  useEffect(() => {
    const mf = field.current;
    if (mf && mf.value !== node.attrs.latex) mf.value = node.attrs.latex ?? "";
    assist.current?.update();
  }, [node.attrs.latex, node.attrs.style, math]);

  const numbered = node.attrs.numbered !== false;
  const multi = isMultiLine(node.attrs.latex ?? "");
  const setLatex = (latex: string) => {
    updateAttributes({ latex });
    const mf = field.current;
    if (mf) {
      mf.value = latex;
      window.setTimeout(() => mf.focus());
    }
  };
  const addRow = (mf: MathfieldElement) => {
    if (!isMultiLine(mf.value)) mf.value = toMultiLine(mf.value);
    mf.focus();
    // The cursor has to be inside the aligned block (in its last row): at the very
    // end, MathLive would wrap everything in \displaylines instead of adding a row.
    mf.executeCommand("moveToMathfieldEnd");
    mf.executeCommand("moveToPreviousChar");
    mf.executeCommand("addRowAfter");
    updateAttributes({ latex: mf.value });
  };
  const addLine = () => {
    if (field.current) addRow(field.current);
    else {
      pending.current = addRow;
      setEditing(true);
    }
  };

  return (
    <NodeViewWrapper
      className={`equation${selected ? " is-selected" : ""}${numbered ? "" : " unnumbered"}`}
      data-equation=""
    >
      {editing ? (
        <div ref={host} className="equation-field" />
      ) : (
        <div
          className="equation-field equation-static"
          onMouseDown={(e) => {
            e.preventDefault();
            setEditing(true);
          }}
          dangerouslySetInnerHTML={{ __html: markup || "&nbsp;" }}
        />
      )}
      {/* The number "(3.1)" comes from CSS counters (see templateStyle.ts). */}
      {numbered && <span className="equation-number" aria-hidden />}
      <div className="equation-tools" contentEditable={false}>
        <label title={t("editor.eqNumberedTitle")}>
          <input type="checkbox" checked={numbered} onChange={(e) => updateAttributes({ numbered: e.target.checked })} />
          {t("editor.eqNumbered")}
        </label>
        <label title={t("editor.eqMultiTitle")}>
          <input
            type="checkbox"
            checked={multi}
            onChange={(e) => setLatex(e.target.checked ? toMultiLine(node.attrs.latex ?? "") : toSingleLine(node.attrs.latex ?? ""))}
          />
          {t("editor.eqMulti")}
        </label>
        <button type="button" onClick={addLine} title={t("editor.eqAddLineTitle")}>
          {t("editor.eqAddLine")}
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
    // `style`: "arabic" or "international" when it isn't the template's way.
    return { latex: { default: "" }, numbered: { default: true }, style: { default: null, rendered: false } };
  },

  parseHTML() {
    return [
      {
        tag: "div[data-equation]",
        getAttrs: (el) => ({
          latex: (el as HTMLElement).dataset.latex ?? "",
          numbered: (el as HTMLElement).dataset.numbered !== "false",
          style: (el as HTMLElement).dataset.mathStyle ?? null,
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
        ...(node.attrs.style ? { "data-math-style": node.attrs.style } : {}),
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
