// Find and replace: a ProseMirror plugin that finds a text in the document (headings,
// paragraphs, captions, table cells, list items…), marks every match, and replaces
// the current one or all of them. Matches never cross from one paragraph into the
// next; inline formulas and citations count as one character a match can't span.
import { Extension, type Editor } from "@tiptap/core";
import { Plugin, PluginKey, TextSelection, type EditorState, type Transaction } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";

export interface Match {
  from: number;
  to: number;
}

export interface SearchState {
  query: string;
  caseSensitive: boolean;
  matches: Match[];
  /** Index of the current match (-1 when there are none). */
  current: number;
}

export const searchKey = new PluginKey<SearchState>("texloomSearch");

const EMPTY: SearchState = { query: "", caseSensitive: false, matches: [], current: -1 };

/** Arabic diacritics (tashkeel, the dagger alef) and tatweel: ignored by loose search. */
const IGNORED = /[ً-ٰٟـ]/;
const FOLDED: Record<string, string> = { "أ": "ا", "إ": "ا", "آ": "ا", "ٱ": "ا", "ة": "ه", "ى": "ي" };

/** Text for loose matching: lower case, without Arabic diacritics and tatweel, with
 *  alef forms, taa marbuta and alef maqsura folded, so "الاسلام" finds "الإسلام" and
 *  "مُحَمَّد". `at[i]` is the index in `s` of the folded text's character `i`. */
export function fold(s: string): { text: string; at: number[] } {
  let text = "";
  const at: number[] = [];
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (IGNORED.test(c)) continue;
    for (const l of (FOLDED[c] ?? c).toLowerCase()) {
      text += l;
      at.push(i);
    }
  }
  return { text, at };
}

/** Every place `query` occurs in a textblock of `doc`. Without `caseSensitive`
 *  ("Match case"), case, Arabic diacritics and letter variants are ignored. */
export function findMatches(doc: PMNode, query: string, caseSensitive: boolean): Match[] {
  const out: Match[] = [];
  if (!query) return out;
  const needle = caseSensitive ? query : fold(query).text;
  if (!needle) return out;
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    // The block's text, with the document position of each character.
    let text = "";
    const at: number[] = [];
    node.forEach((child, offset) => {
      if (child.isText && child.text) {
        for (let i = 0; i < child.text.length; i++) at.push(pos + 1 + offset + i);
        text += child.text;
      } else {
        // An inline node (formula, citation, reference): one character no match spans.
        at.push(pos + 1 + offset);
        text += "￼";
      }
    });
    if (caseSensitive) {
      for (let i = text.indexOf(needle); i >= 0; i = text.indexOf(needle, i + needle.length)) {
        out.push({ from: at[i], to: at[i + needle.length - 1] + 1 });
      }
    } else {
      const loose = fold(text);
      for (let i = loose.text.indexOf(needle); i >= 0; i = loose.text.indexOf(needle, i + needle.length)) {
        let last = loose.at[i + needle.length - 1];
        // The last letter's diacritics belong to the match (so replacing doesn't leave them).
        while (last + 1 < text.length && IGNORED.test(text[last + 1])) last++;
        out.push({ from: at[loose.at[i]], to: at[last] + 1 });
      }
    }
    return false;
  });
  return out;
}

type Meta = { query?: string; caseSensitive?: boolean; current?: number };

export const Search = Extension.create({
  name: "texloomSearch",
  addProseMirrorPlugins() {
    return [
      new Plugin<SearchState>({
        key: searchKey,
        state: {
          init: () => EMPTY,
          apply(tr: Transaction, prev: SearchState): SearchState {
            const meta = tr.getMeta(searchKey) as Meta | undefined;
            if (!meta && !tr.docChanged) return prev;
            const query = meta?.query ?? prev.query;
            const caseSensitive = meta?.caseSensitive ?? prev.caseSensitive;
            const matches = findMatches(tr.doc, query, caseSensitive);
            let current = meta?.current ?? prev.current;
            if (meta?.query !== undefined && meta.current === undefined) {
              // A new search starts at the first match after the cursor.
              const i = matches.findIndex((m) => m.from >= tr.selection.from);
              current = matches.length ? (i < 0 ? 0 : i) : -1;
            }
            current = matches.length ? Math.min(Math.max(current, 0), matches.length - 1) : -1;
            return { query, caseSensitive, matches, current };
          },
        },
        props: {
          decorations(state: EditorState) {
            const s = searchKey.getState(state);
            if (!s?.matches.length) return null;
            return DecorationSet.create(
              state.doc,
              s.matches.map((m, i) => Decoration.inline(m.from, m.to, { class: i === s.current ? "search-match current" : "search-match" })),
            );
          },
        },
      }),
    ];
  },
});

export const searchState = (editor: Editor): SearchState => searchKey.getState(editor.state) ?? EMPTY;

/** Starts or changes a search. */
export function setSearch(editor: Editor, query: string, caseSensitive: boolean) {
  editor.view.dispatch(editor.state.tr.setMeta(searchKey, { query, caseSensitive } satisfies Meta));
  revealCurrent(editor);
}

/** Selects and scrolls to the current match (focus stays in the search field). */
function revealCurrent(editor: Editor) {
  const s = searchState(editor);
  const m = s.matches[s.current];
  if (!m) return;
  const { state, view } = editor;
  view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, m.from, m.to)).scrollIntoView());
}

/** Moves to the next (1) or previous (-1) match, wrapping around. */
export function stepMatch(editor: Editor, dir: 1 | -1) {
  const s = searchState(editor);
  if (!s.matches.length) return;
  const current = (s.current + dir + s.matches.length) % s.matches.length;
  editor.view.dispatch(editor.state.tr.setMeta(searchKey, { current } satisfies Meta));
  revealCurrent(editor);
}

/** Replaces the current match and moves to the next one. */
export function replaceCurrent(editor: Editor, text: string) {
  const s = searchState(editor);
  const m = s.matches[s.current];
  if (!m) return;
  const tr = editor.state.tr;
  if (text) tr.insertText(text, m.from, m.to);
  else tr.delete(m.from, m.to);
  // The same index is now the next match (the replaced one is gone, unless the
  // replacement contains the search text: then skip past it).
  const again = findMatches(tr.doc, s.query, s.caseSensitive);
  const end = m.from + text.length;
  const next = again.findIndex((x) => x.from >= end);
  tr.setMeta(searchKey, { current: next < 0 ? 0 : next } satisfies Meta);
  editor.view.dispatch(tr);
  revealCurrent(editor);
}

/** Replaces every match in one step (one undo). Returns how many were replaced. */
export function replaceAll(editor: Editor, text: string): number {
  const s = searchState(editor);
  if (!s.matches.length) return 0;
  const tr = editor.state.tr;
  // From the end, so earlier positions stay valid.
  for (const m of [...s.matches].reverse()) {
    if (text) tr.insertText(text, m.from, m.to);
    else tr.delete(m.from, m.to);
  }
  editor.view.dispatch(tr);
  return s.matches.length;
}
