// Raw LaTeX block: code written into the document exactly as typed. The escape hatch
// for anything the editor can't do visually yet (TikZ drawings, special tables…).
import { useLayoutEffect, useRef } from "react";
import { Node, mergeAttributes, NodeViewWrapper, ReactNodeViewRenderer, type ReactNodeViewProps } from "@tiptap/react";
import { useT } from "../i18n";

declare module "@tiptap/react" {
  interface Commands<ReturnType> {
    rawLatex: { insertRawLatex: (latex?: string) => ReturnType };
  }
}

function RawLatexView({ node, updateAttributes, selected, editor, getPos }: ReactNodeViewProps) {
  const t = useT();
  const area = useRef<HTMLTextAreaElement>(null);
  const latex: string = node.attrs.latex ?? "";

  // Grow with the content instead of scrolling inside the page.
  useLayoutEffect(() => {
    const el = area.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [latex]);

  // New blocks start empty: put the cursor straight into them.
  useLayoutEffect(() => {
    if (!latex) area.current?.focus();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const leave = (after: boolean) => {
    const pos = typeof getPos === "function" ? getPos() : undefined;
    if (pos === undefined) return;
    editor.chain().focus().setTextSelection(after ? pos + node.nodeSize : Math.max(0, pos - 1)).run();
  };

  const onKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const el = e.currentTarget;
    if (e.key === "Escape") {
      e.preventDefault();
      leave(true);
    } else if (e.key === "Tab") {
      // Indent instead of leaving the field.
      e.preventDefault();
      el.setRangeText("  ", el.selectionStart, el.selectionEnd, "end");
      updateAttributes({ latex: el.value });
    } else if (e.key === "ArrowDown" && el.selectionStart === el.value.length) {
      e.preventDefault();
      leave(true);
    } else if (e.key === "ArrowUp" && el.selectionEnd === 0) {
      e.preventDefault();
      leave(false);
    }
  };

  return (
    <NodeViewWrapper className={`raw-latex${selected ? " is-selected" : ""}`} data-raw-latex="">
      <div className="raw-latex-label" contentEditable={false} data-drag-handle="">
        LaTeX <span>{t("editor.rawLatexNote")}</span>
      </div>
      <textarea
        ref={area}
        value={latex}
        rows={1}
        spellCheck={false}
        placeholder={"\\begin{center}\n  …\n\\end{center}"}
        onChange={(e) => updateAttributes({ latex: e.target.value })}
        onKeyDown={onKeyDown}
      />
    </NodeViewWrapper>
  );
}

export const RawLatex = Node.create({
  name: "rawLatex",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,

  addAttributes() {
    return { latex: { default: "" } };
  },

  parseHTML() {
    return [{ tag: "div[data-raw-latex]", getAttrs: (el) => ({ latex: (el as HTMLElement).dataset.latex ?? "" }) }];
  },

  renderHTML({ node, HTMLAttributes }) {
    return ["div", mergeAttributes(HTMLAttributes, { "data-raw-latex": "", "data-latex": node.attrs.latex })];
  },

  addNodeView() {
    // Typing in the textarea belongs to it, not to ProseMirror.
    return ReactNodeViewRenderer(RawLatexView, { stopEvent: ({ event }) => event.target instanceof HTMLTextAreaElement });
  },

  addCommands() {
    return {
      insertRawLatex:
        (latex = "") =>
        ({ commands }) =>
          commands.insertContent({ type: this.name, attrs: { latex } }),
    };
  },
});
