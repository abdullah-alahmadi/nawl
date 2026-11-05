// Inline math: a small MathLive field inside a sentence ("where D_t is the deficit").
// Stored as LaTeX in the `latex` attribute; written to the PDF as $…$.
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
import { TextSelection } from "@tiptap/pm/state";

declare module "@tiptap/react" {
  interface Commands<ReturnType> {
    inlineMath: {
      /** Inserts inline math at the cursor (or turns the selected text into it). */
      insertInlineMath: (latex?: string) => ReturnType;
    };
  }
}

function InlineMathView({ node, updateAttributes, editor, getPos, selected }: ReactNodeViewProps) {
  const host = useRef<HTMLSpanElement>(null);
  const field = useRef<MathfieldElement | null>(null);

  useEffect(() => {
    const mf = new MathfieldElement();
    mf.mathVirtualKeyboardPolicy = "manual";
    mf.value = node.attrs.latex ?? "";
    mf.addEventListener("input", () => updateAttributes({ latex: mf.value }));
    // Leaving with the arrow keys puts the text cursor just before or after the formula.
    mf.addEventListener("move-out", (ev) => {
      const pos = typeof getPos === "function" ? getPos() : undefined;
      if (pos === undefined) return;
      const dir = (ev as CustomEvent<{ direction: string }>).detail.direction;
      const after = dir !== "backward" && dir !== "upward";
      editor
        .chain()
        .focus()
        .command(({ tr }) => {
          tr.setSelection(TextSelection.create(tr.doc, after ? pos + node.nodeSize : pos));
          return true;
        })
        .run();
    });
    // Enter or Escape finishes the formula and continues the sentence.
    mf.addEventListener("keydown", (ev) => {
      if (ev.key === "Enter" || ev.key === "Escape") {
        ev.preventDefault();
        const pos = typeof getPos === "function" ? getPos() : undefined;
        if (pos !== undefined) editor.chain().focus().setTextSelection(pos + node.nodeSize).run();
      }
    });
    host.current?.appendChild(mf);
    field.current = mf;
    if (!mf.value) requestAnimationFrame(() => mf.focus());
    return () => mf.remove();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const mf = field.current;
    if (mf && mf.value !== node.attrs.latex) mf.value = node.attrs.latex ?? "";
  }, [node.attrs.latex]);

  return (
    <NodeViewWrapper as="span" className={`inline-math${selected ? " is-selected" : ""}`} data-inline-math="">
      <span ref={host} />
    </NodeViewWrapper>
  );
}

export const InlineMath = Node.create({
  name: "inlineMath",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return { latex: { default: "" } };
  },

  parseHTML() {
    return [{ tag: "span[data-inline-math]", getAttrs: (el) => ({ latex: (el as HTMLElement).dataset.latex ?? "" }) }];
  },

  renderHTML({ node, HTMLAttributes }) {
    return ["span", mergeAttributes(HTMLAttributes, { "data-inline-math": "", "data-latex": node.attrs.latex })];
  },

  addNodeView() {
    // Keystrokes inside the formula belong to MathLive, not ProseMirror.
    return ReactNodeViewRenderer(InlineMathView, { as: "span", stopEvent: () => true });
  },

  addCommands() {
    return {
      insertInlineMath:
        (latex?: string) =>
        ({ state, chain }) => {
          const { from, to, empty } = state.selection;
          const text = latex ?? (empty ? "" : state.doc.textBetween(from, to, " "));
          return chain().insertContentAt({ from, to }, { type: this.name, attrs: { latex: text } }).run();
        },
    };
  },

  addKeyboardShortcuts() {
    return { "Mod-m": () => this.editor.commands.insertInlineMath() };
  },

  addInputRules() {
    // Typing $x^2$ turns it into inline math (a lone "$" or "$$" does not).
    return [
      nodeInputRule({
        find: /(?:^|\s)(\$([^$\s](?:[^$]*[^$\s])?)\$)$/,
        type: this.type,
        getAttributes: (match) => ({ latex: match[2] }),
      }),
    ];
  },
});
