// No runs of empty lines, as in LyX: an empty line in LaTeX only ends a paragraph, so
// extra ones would show on screen but never in the PDF. Enter on an empty line does
// nothing, and runs of empty paragraphs (from pasting or deleting) shrink to one.
import { Extension } from "@tiptap/react";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { Node as PMNode } from "@tiptap/pm/model";

const isEmptyParagraph = (n: PMNode | null | undefined) => n?.type.name === "paragraph" && n.content.size === 0;

export const EmptyLines = Extension.create({
  name: "emptyLines",
  // Before the list and paragraph Enter handlers.
  priority: 1000,

  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => {
        const { selection, doc } = editor.state;
        const { $from, empty } = selection;
        if (!empty || $from.parent.type.name !== "paragraph") return false;
        // Enter in an empty list item leaves the list: keep that.
        if ($from.node($from.depth - 1).type.name === "listItem") return false;
        const para = $from.parent;
        if (para.content.size === 0) return true;
        // At the end of a line with an empty line below: go there instead of adding one.
        if ($from.parentOffset === para.content.size) {
          const after = $from.after();
          if (isEmptyParagraph(doc.resolve(after).nodeAfter)) return editor.commands.setTextSelection(after + 1);
        }
        // At the start of a line with an empty line above: go there.
        if ($from.parentOffset === 0) {
          const before = $from.before();
          if (isEmptyParagraph(doc.resolve(before).nodeBefore)) return editor.commands.setTextSelection(before - 1);
        }
        return false;
      },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("empty-lines"),
        appendTransaction: (transactions, _old, state) => {
          if (!transactions.some((tr) => tr.docChanged)) return null;
          const cursor = state.selection.$from.pos;
          const doomed: { from: number; to: number }[] = [];
          // In each run of empty paragraphs keep one: the one with the cursor, if any.
          const collapse = (parent: PMNode, contentStart: number) => {
            let kept: { node: PMNode; pos: number } | null = null;
            parent.forEach((child, offset) => {
              const pos = contentStart + offset;
              if (!isEmptyParagraph(child)) {
                kept = null;
                return;
              }
              if (!kept) {
                kept = { node: child, pos };
                return;
              }
              const victim = cursor === pos + 1 ? kept : { node: child, pos };
              doomed.push({ from: victim.pos, to: victim.pos + victim.node.nodeSize });
              if (victim === kept) kept = { node: child, pos };
            });
          };
          collapse(state.doc, 0);
          state.doc.descendants((node, pos) => {
            if (node.isTextblock || node.isAtom) return false;
            collapse(node, pos + 1);
            return true;
          });
          if (doomed.length === 0) return null;
          const tr = state.tr;
          doomed.sort((a, b) => b.from - a.from).forEach((r) => tr.delete(r.from, r.to));
          return tr.setMeta("addToHistory", false);
        },
      }),
    ];
  },
});
