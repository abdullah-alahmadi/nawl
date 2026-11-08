// Inline math: a small MathLive field inside a sentence ("where D_t is the deficit").
// Stored as LaTeX in the `latex` attribute; written to the PDF as $…$.
import { MathfieldElement, convertLatexToMarkup } from "mathlive";
import "mathlive/fonts.css";
import "mathlive/static.css";
import { InputRule, Node, mergeAttributes, type Editor } from "@tiptap/react";
import { TextSelection } from "@tiptap/pm/state";
import type { Node as PMNode } from "@tiptap/pm/model";
import type { NodeView } from "@tiptap/pm/view";

declare module "@tiptap/react" {
  interface Commands<ReturnType> {
    inlineMath: {
      /** Inserts inline math at the cursor (or turns the selected text into it). */
      insertInlineMath: (latex?: string) => ReturnType;
    };
  }
}

/** A formula as static markup (MathLive's own rendering, so it looks as when edited);
 *  `display` for an equation of its own (TeX's display style). */
export function staticMarkup(latex: string, display = false): string {
  try {
    return convertLatexToMarkup(latex, display ? { defaultMode: "math" } : undefined);
  } catch {
    return latex.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c] ?? c);
  }
}

/**
 * A plain (non-React) node view: static markup, and a live MathLive field only while
 * the formula is being edited (a new, empty one starts so). Thousands of live fields
 * froze a long book for minutes, and thousands of React node views still took seconds
 * to mount, so this one builds its own DOM.
 */
class InlineMathView implements NodeView {
  dom: HTMLSpanElement;
  private field: MathfieldElement | null = null;
  private stopListening: (() => void) | null = null;

  constructor(
    private node: PMNode,
    private editor: Editor,
    private getPos: () => number | undefined,
  ) {
    this.dom = document.createElement("span");
    this.dom.className = "inline-math";
    this.dom.setAttribute("data-inline-math", "");
    if (node.attrs.latex) this.showStatic();
    else this.startEditing();
  }

  private showStatic() {
    this.stopEditing();
    const span = document.createElement("span");
    span.className = "inline-math-static";
    span.innerHTML = staticMarkup(this.node.attrs.latex ?? "") || "&nbsp;";
    span.addEventListener("mousedown", (e) => {
      e.preventDefault();
      this.startEditing();
    });
    this.dom.replaceChildren(span);
  }

  private setLatex(latex: string) {
    const pos = this.getPos();
    if (pos === undefined) return;
    this.editor.view.dispatch(this.editor.state.tr.setNodeMarkup(pos, undefined, { ...this.node.attrs, latex }));
  }

  private startEditing() {
    if (this.field) return;
    const { editor } = this;
    const mf = new MathfieldElement();
    mf.mathVirtualKeyboardPolicy = "manual";
    mf.value = this.node.attrs.latex ?? "";
    mf.addEventListener("input", () => this.setLatex(mf.value));
    // Leaving with the arrow keys puts the text cursor just before or after the formula.
    mf.addEventListener("move-out", (ev) => {
      const pos = this.getPos();
      if (pos === undefined) return;
      const dir = (ev as CustomEvent<{ direction: string }>).detail.direction;
      const after = dir !== "backward" && dir !== "upward";
      editor
        .chain()
        .focus()
        .command(({ tr }) => {
          tr.setSelection(TextSelection.create(tr.doc, after ? pos + this.node.nodeSize : pos));
          return true;
        })
        .run();
    });
    // Enter or Escape finishes the formula and continues the sentence.
    mf.addEventListener("keydown", (ev) => {
      if (ev.key === "Enter" || ev.key === "Escape") {
        ev.preventDefault();
        const pos = this.getPos();
        if (pos !== undefined) editor.chain().focus().setTextSelection(pos + this.node.nodeSize).run();
      }
    });
    // Back to static markup once the field loses the focus (by any of the above, or a
    // click), or when the text cursor goes elsewhere (should a blur be missed). Later:
    // switching the DOM inside a focus or transaction handler upsets both.
    const done = () => window.setTimeout(() => this.field === mf && this.showStatic());
    mf.addEventListener("focusout", done);
    mf.addEventListener("blur", done);
    const elsewhere = () => {
      if (document.activeElement !== mf) done();
    };
    editor.on("selectionUpdate", elsewhere);
    this.stopListening = () => editor.off("selectionUpdate", elsewhere);
    this.field = mf;
    this.dom.replaceChildren(mf);
    // Once attached (a timer, not an animation frame: hidden windows run no frames).
    window.setTimeout(() => mf.focus());
  }

  private stopEditing() {
    this.stopListening?.();
    this.stopListening = null;
    this.field = null;
  }

  update(node: PMNode) {
    if (node.type !== this.node.type) return false;
    const changed = node.attrs.latex !== this.node.attrs.latex;
    this.node = node;
    if (this.field) {
      if (this.field.value !== node.attrs.latex) this.field.value = node.attrs.latex ?? "";
    } else if (changed) {
      this.showStatic();
    }
    return true;
  }

  selectNode() {
    this.dom.classList.add("is-selected");
  }

  deselectNode() {
    this.dom.classList.remove("is-selected");
  }

  // Keystrokes and clicks inside the formula belong to MathLive, not ProseMirror.
  stopEvent() {
    return true;
  }

  ignoreMutation() {
    return true;
  }

  destroy() {
    this.stopEditing();
  }
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
    return ({ node, editor, getPos }) => new InlineMathView(node, editor, getPos);
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
    // Not nodeInputRule: TipTap now puts the typed character in the document before
    // running the rule, but nodeInputRule still inserts it again, which left a stray
    // "$" after the formula. Here the range covers the matched text, typed "$" included.
    return [
      new InputRule({
        find: /(?:^|\s)(\$([^$\s](?:[^$]*[^$\s])?)\$)$/,
        handler: ({ state, range, match }) => {
          const start = range.from + match[0].lastIndexOf(match[1]);
          state.tr.replaceWith(start, range.to, this.type.create({ latex: match[2] }));
        },
      }),
    ];
  },
});
