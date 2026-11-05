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

  return (
    <NodeViewWrapper className={`equation${selected ? " is-selected" : ""}`} data-equation="">
      <div ref={host} className="equation-field" />
      <span className="equation-number" aria-hidden>(#)</span>
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
    return { latex: { default: "" } };
  },

  parseHTML() {
    return [{ tag: "div[data-equation]", getAttrs: (el) => ({ latex: (el as HTMLElement).dataset.latex ?? "" }) }];
  },

  renderHTML({ node, HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-equation": "", "data-latex": node.attrs.latex })];
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
