// Text direction per paragraph: an English paragraph in an Arabic document, or an
// Arabic one in an English document. The page takes the document's direction; a
// paragraph or heading only stores `dir` when it differs (the LaTeX output sets it
// in the other language, see crates/codegen/src/language.rs).
import { Extension } from "@tiptap/core";

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    direction: {
      /** Switches the selected paragraphs and headings between left to right and right to left. */
      toggleDirection: () => ReturnType;
    };
  }
  interface Storage {
    direction: { rtl: boolean };
  }
}

const TYPES = ["paragraph", "heading"];

export const Direction = Extension.create({
  name: "direction",

  addStorage() {
    return { rtl: false };
  },

  addGlobalAttributes() {
    return [
      {
        types: TYPES,
        attributes: {
          dir: {
            default: null,
            parseHTML: (el) => {
              const d = el.getAttribute("dir");
              return d === "ltr" || d === "rtl" ? d : null;
            },
            renderHTML: (attrs) => (attrs.dir ? { dir: attrs.dir } : {}),
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      toggleDirection:
        () =>
        ({ state, tr, dispatch }) => {
          const docDir = this.storage.rtl ? "rtl" : "ltr";
          const { from, to } = state.selection;
          let found = false;
          state.doc.nodesBetween(from, to, (node, pos) => {
            if (!TYPES.includes(node.type.name)) return true;
            found = true;
            if (dispatch) {
              const current = node.attrs.dir ?? docDir;
              const next = current === "rtl" ? "ltr" : "rtl";
              // Back to the document's direction: no attribute at all.
              tr.setNodeMarkup(pos, undefined, { ...node.attrs, dir: next === docDir ? null : next });
            }
            return false;
          });
          return found;
        },
    };
  },

  addKeyboardShortcuts() {
    return { "Mod-Shift-x": () => this.editor.commands.toggleDirection() };
  },
});
