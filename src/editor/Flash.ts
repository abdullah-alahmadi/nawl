// Marks a block briefly (after jumping to it from the PDF or the error list). A node
// decoration, not a class on the DOM element: ProseMirror may redraw the element
// right after the jump, which would drop a class set by hand.
import { Extension, type Editor } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

const flashKey = new PluginKey<DecorationSet>("nawlFlash");

export const Flash = Extension.create({
  name: "nawlFlash",
  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: flashKey,
        state: {
          init: () => DecorationSet.empty,
          apply(tr, set) {
            const meta = tr.getMeta(flashKey) as { from: number; to: number } | null | undefined;
            if (meta === null) return DecorationSet.empty;
            if (meta) return DecorationSet.create(tr.doc, [Decoration.node(meta.from, meta.to, { class: "sync-flash" })]);
            return set.map(tr.mapping, tr.doc);
          },
        },
        props: { decorations: (state) => flashKey.getState(state) },
      }),
    ];
  },
});

let timer: number | undefined;

/** Marks the top-level node at `pos` for a moment. */
export function flashBlock(editor: Editor, pos: number) {
  const node = editor.state.doc.nodeAt(pos);
  if (!node) return;
  window.clearTimeout(timer);
  editor.view.dispatch(editor.state.tr.setMeta(flashKey, { from: pos, to: pos + node.nodeSize }));
  timer = window.setTimeout(() => {
    if (!editor.isDestroyed) editor.view.dispatch(editor.state.tr.setMeta(flashKey, null));
  }, 1700);
}
