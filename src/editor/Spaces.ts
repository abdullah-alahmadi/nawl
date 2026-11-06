// One space between words, as in LyX: LaTeX prints any run of spaces as one, and
// drops spaces at the start of a paragraph, so extra ones would show on screen but
// never in the PDF. Typing a space next to a space (or at the start of a line) does
// nothing, and pasted text has its runs of spaces shrunk to one. Code (code blocks,
// inline code) keeps every space.
import { Extension } from "@tiptap/react";
import { Plugin, PluginKey, type EditorState } from "@tiptap/pm/state";
import { Fragment, Slice, type Node as PMNode } from "@tiptap/pm/model";

/** The cursor is in code, where spaces are kept as typed. */
function inCode(state: EditorState, from: number): boolean {
  const $from = state.doc.resolve(from);
  if ($from.parent.type.spec.code) return true;
  const marks = state.storedMarks ?? $from.marks();
  return marks.some((m) => m.type.name === "code");
}

/** Pasted content with runs of spaces shrunk to one and none at a line's start.
 *  `keepStart` holds for the first line when it joins the line being pasted into. */
function tidy(fragment: Fragment, code: boolean, keepStart: { first: boolean }): Fragment {
  const nodes: PMNode[] = [];
  fragment.forEach((node) => {
    if (node.isText) {
      if (code || node.marks.some((m) => m.type.name === "code")) nodes.push(node);
      else {
        const text = (node.text ?? "").replace(/[ \t ]{2,}/g, " ").replace(/\t/g, " ");
        if (text) nodes.push(node.type.schema.text(text, node.marks));
      }
      return;
    }
    const isCode = code || !!node.type.spec.code;
    let content = tidy(node.content, isCode, keepStart);
    if (node.isTextblock) {
      if (!isCode && !keepStart.first) content = trimStart(content);
      keepStart.first = false;
    }
    nodes.push(node.copy(content));
  });
  return Fragment.fromArray(nodes);
}

/** Drops spaces at the start of a textblock's content. */
function trimStart(content: Fragment): Fragment {
  const first = content.firstChild;
  if (!first?.isText || first.marks.some((m) => m.type.name === "code")) return content;
  const text = (first.text ?? "").replace(/^[ \t]+/, "");
  const rest: PMNode[] = [];
  content.forEach((n, _o, i) => i > 0 && rest.push(n));
  return Fragment.fromArray(text ? [first.type.schema.text(text, first.marks), ...rest] : rest);
}

export const Spaces = Extension.create({
  name: "spaces",
  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("single-spaces"),
        props: {
          handleTextInput(view, from, to, text) {
            if (!text.includes(" ") || inCode(view.state, from)) return false;
            const $from = view.state.doc.resolve(from);
            const $to = view.state.doc.resolve(to);
            if (!$from.parent.isTextblock || $from.parent !== $to.parent) return false;
            const start = $from.parentOffset;
            const end = $to.parentOffset;
            const char = (a: number, b: number) => (a >= 0 && b <= $from.parent.content.size ? $from.parent.textBetween(a, b, undefined, "\ufffc") : "");
            const before = char(start - 1, start);
            const after = char(end, end + 1);
            // No runs of spaces, none at a line's start, none next to a space.
            let tidy = text.replace(/ {2,}/g, " ");
            if (start === 0 || before === " ") tidy = tidy.replace(/^ +/, "");
            if (after === " ") tidy = tidy.replace(/ +$/, "");
            if (tidy === text) return false;
            if (tidy) view.dispatch(view.state.tr.insertText(tidy, from, to).scrollIntoView());
            else if (from !== to) view.dispatch(view.state.tr.delete(from, to));
            return true;
          },
          transformPasted(slice, view) {
            const code = inCode(view.state, view.state.selection.from);
            // An open start: the first pasted line continues the current one.
            return new Slice(tidy(slice.content, code, { first: slice.openStart > 0 }), slice.openStart, slice.openEnd);
          },
        },
      }),
    ];
  },
});
