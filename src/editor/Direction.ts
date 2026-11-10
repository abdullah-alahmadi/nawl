// Text direction per paragraph: an English paragraph in an Arabic document, or an
// Arabic one in an English document. The page takes the document's direction; a
// paragraph or heading only stores `dir` when it differs (the LaTeX output sets it
// in the other language, see crates/codegen/src/language.rs).
// Alignment too: a paragraph stores `align` (left, center or right, as seen on the
// page) when it isn't the template's, as for a signature or a centred line.
import { Extension } from "@tiptap/core";

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    direction: {
      /** Switches the selected paragraphs and headings between left to right and right to left. */
      toggleDirection: () => ReturnType;
      /** Aligns the selected paragraphs; the alignment they already have (or null) goes back to the template's. */
      toggleAlign: (align: "left" | "center" | "right" | null) => ReturnType;
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
      {
        types: ["paragraph"],
        attributes: {
          align: {
            default: null,
            parseHTML: (el) => {
              const a = el.getAttribute("data-align") ?? el.style.textAlign;
              return a === "left" || a === "center" || a === "right" ? a : null;
            },
            renderHTML: (attrs) => (attrs.align ? { "data-align": attrs.align } : {}),
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
      toggleAlign:
        (align) =>
        ({ state, tr, dispatch }) => {
          const { from, to } = state.selection;
          const paragraphs: { pos: number; attrs: Record<string, unknown> }[] = [];
          state.doc.nodesBetween(from, to, (node, pos) => {
            if (node.type.name === "paragraph") paragraphs.push({ pos, attrs: node.attrs });
            return true;
          });
          if (!paragraphs.length) return false;
          // All of them already have it: back to the template's alignment.
          const next = paragraphs.every((p) => p.attrs.align === align) ? null : align;
          if (dispatch) for (const p of paragraphs) tr.setNodeMarkup(p.pos, undefined, { ...p.attrs, align: next });
          return true;
        },
    };
  },

  addKeyboardShortcuts() {
    // No shortcuts for alignment: Ctrl+Shift+E exports, Ctrl+J shows the place in the
    // PDF, and Ctrl+Shift+R reloads the page when the editor hasn't the focus.
    return { "Mod-Shift-x": () => this.editor.commands.toggleDirection() };
  },
});
