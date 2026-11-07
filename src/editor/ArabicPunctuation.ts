// Arabic punctuation as you type: in Arabic text "?" becomes "؟", "," "،", ";" "؛",
// and '"' opens or closes «guillemets». English inside Arabic keeps its own marks:
// the character before the cursor decides (an Arabic letter converts; a digit or a
// Latin letter doesn't). Settings → Editor turns it off.
import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";

declare module "@tiptap/core" {
  interface Storage {
    arabicPunctuation: { enabled: boolean };
  }
}

const ARABIC = /[؀-ۿݐ-ݿࢠ-ࣿﭐ-﷿ﹰ-﻿]/;
const MARKS: Record<string, string> = { ",": "،", "?": "؟", ";": "؛" };

export const ArabicPunctuation = Extension.create({
  name: "arabicPunctuation",

  addStorage() {
    return { enabled: true };
  },

  addProseMirrorPlugins() {
    const storage = this.storage;
    const editor = this.editor;
    return [
      new Plugin({
        key: new PluginKey("arabicPunctuation"),
        props: {
          handleTextInput(view, from, to, text) {
            if (!storage.enabled || !(text in MARKS || text === '"')) return false;
            const $from = view.state.doc.resolve(from);
            const block = $from.parent;
            if (block.type.spec.code || $from.marks().some((m) => m.type.spec.code)) return false;
            const before = block.textBetween(Math.max(0, $from.parentOffset - 1), $from.parentOffset, undefined, "￼");
            const dir = block.attrs.dir as string | null | undefined;
            const rtl = dir === "rtl" || (!dir && editor.storage.direction?.rtl === true);
            // Start of a word: nothing, a space or an opening bracket before the cursor.
            const boundary = before === "" || /[\s(\[{«]/.test(before);
            let out: string | undefined;
            if (text === '"') {
              if (ARABIC.test(before) || rtl) out = boundary ? "«" : "»";
            } else if (ARABIC.test(before) || (rtl && (boundary || /[)\]»]/.test(before)))) {
              out = MARKS[text];
            }
            if (!out) return false;
            view.dispatch(view.state.tr.insertText(out, from, to));
            return true;
          },
        },
      }),
    ];
  },
});
