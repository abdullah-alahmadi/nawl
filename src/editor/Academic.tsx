// Cross-references, citations, appendices — and the stable ids that references
// point at. On-screen numbers come from numbering.ts; the PDF uses \ref and BibTeX.
import { Extension, Node, mergeAttributes, NodeViewWrapper, ReactNodeViewRenderer, useEditorState } from "@tiptap/react";
import type { Editor, ReactNodeViewProps } from "@tiptap/react";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { Template } from "../api";
import type { BibEntry } from "../api";
import { computeNumbering, type NumberedItem } from "./numbering";

/** Node types that cross-references can point at. */
const TARGET_TYPES = ["heading", "figure", "tableFigure", "equation"];

const newId = () => Math.random().toString(36).slice(2, 10);

/** What node views need from the app; changed with `setTexloomContext`. */
export interface TexloomContext {
  template: Template | null;
  bibEntries: BibEntry[];
}

declare module "@tiptap/react" {
  interface Storage {
    texloom: TexloomContext;
  }
  interface Commands<ReturnType> {
    academic: {
      insertRef: (target: string) => ReturnType;
      insertCitation: (key: string) => ReturnType;
      insertAppendices: () => ReturnType;
    };
  }
}

/**
 * Gives headings, figures, tables and equations a stable `id`, and keeps ids unique
 * (copy-paste duplicates them). References store these ids, so they survive edits.
 */
export const TexloomIds = Extension.create<object, TexloomContext>({
  name: "texloom",

  addStorage() {
    return { template: null, bibEntries: [] };
  },

  addGlobalAttributes() {
    return [
      {
        types: TARGET_TYPES,
        attributes: {
          id: {
            default: null,
            parseHTML: (el) => el.getAttribute("data-id"),
            renderHTML: (attrs) => (attrs.id ? { "data-id": attrs.id } : {}),
          },
        },
      },
    ];
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("texloom-ids"),
        appendTransaction: (transactions, _old, state) => {
          if (!transactions.some((tr) => tr.docChanged) && _old.doc.eq(state.doc)) return null;
          const seen = new Set<string>();
          const tr = state.tr;
          state.doc.descendants((node, pos) => {
            if (!TARGET_TYPES.includes(node.type.name)) return true;
            const id = node.attrs.id as string | null;
            if (!id || seen.has(id)) {
              const fresh = newId();
              seen.add(fresh);
              tr.setNodeMarkup(pos, undefined, { ...node.attrs, id: fresh });
            } else {
              seen.add(id);
            }
            return node.type.name !== "heading";
          });
          return tr.docChanged ? tr.setMeta("addToHistory", false) : null;
        },
      }),
    ];
  },
});

/** Updates what node views read (template, bibliography) and redraws them. */
export function setTexloomContext(editor: Editor, ctx: Partial<TexloomContext>) {
  Object.assign(editor.storage.texloom, ctx);
  editor.view.dispatch(editor.state.tr.setMeta("texloom-context", true));
}

/** Numbered targets of the current document (empty until a template is set). */
export function useNumbering(editor: Editor): NumberedItem[] {
  return useEditorState({
    editor,
    selector: ({ editor }) => {
      const t = editor.storage.texloom.template;
      return t ? computeNumbering(editor.state.doc, t) : [];
    },
    equalityFn: (a, b) => JSON.stringify(a) === JSON.stringify(b),
  });
}

// ---------------------------------------------------------------- cross-references

function RefView({ node, editor, selected }: ReactNodeViewProps) {
  const items = useNumbering(editor);
  const target = items.find((i) => i.id === node.attrs.target);
  const jump = (e: React.MouseEvent) => {
    if (!target || !(e.ctrlKey || e.metaKey)) return;
    e.preventDefault();
    editor.chain().focus().setTextSelection(target.pos + 1).scrollIntoView().run();
  };
  return (
    <NodeViewWrapper
      as="span"
      className={`ref${target ? "" : " broken"}${selected ? " is-selected" : ""}`}
      title={target ? `${target.refText}: ${target.text} (Ctrl+click to go there)` : "The item this pointed to was deleted"}
      onClick={jump}
    >
      {target ? target.refText : "??"}
    </NodeViewWrapper>
  );
}

export const CrossRef = Node.create({
  name: "ref",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return { target: { default: "" } };
  },
  parseHTML: () => [{ tag: "span[data-ref]", getAttrs: (el) => ({ target: (el as HTMLElement).dataset.ref }) }],
  renderHTML: ({ node, HTMLAttributes }) => ["span", mergeAttributes(HTMLAttributes, { "data-ref": node.attrs.target })],
  addNodeView() {
    return ReactNodeViewRenderer(RefView, { as: "span" });
  },
  addCommands() {
    return {
      insertRef:
        (target: string) =>
        ({ commands }) =>
          commands.insertContent([{ type: this.name, attrs: { target } }, { type: "text", text: " " }]),
    };
  },
});

// ---------------------------------------------------------------- citations

/** On-screen citation labels, matching the template's bibliography style. */
export function citationLabels(editor: Editor): Map<string, string> {
  const { template, bibEntries } = editor.storage.texloom;
  const labels = new Map<string, string>();
  if (!template) return labels;
  const byKey = new Map(bibEntries.map((e) => [e.key, e]));
  const order: string[] = [];
  editor.state.doc.descendants((n) => {
    if (n.type.name === "citation") for (const k of n.attrs.keys as string[]) if (!order.includes(k)) order.push(k);
  });
  const style = template.bibliography.style;
  if (style === "authorYear") {
    for (const k of order) {
      const e = byKey.get(k);
      if (!e) continue;
      const a = e.authors;
      const who = a.length === 0 ? e.title || k : a.length === 1 ? a[0] : a.length === 2 ? `${a[0]} and ${a[1]}` : `${a[0]} et al.`;
      labels.set(k, `${who}, ${e.year ?? "n.d."}`);
    }
  } else {
    // IEEE numbers by first citation; "numeric" sorts the list by author, then year, then title.
    let known = order.filter((k) => byKey.has(k));
    if (style === "numeric") {
      const sortKey = (k: string) => {
        const e = byKey.get(k)!;
        return `${(e.authors[0] ?? e.title).toLowerCase()}\u0000${e.year ?? ""}\u0000${e.title.toLowerCase()}`;
      };
      known = [...known].sort((x, y) => sortKey(x).localeCompare(sortKey(y)));
    }
    known.forEach((k, i) => labels.set(k, String(i + 1)));
  }
  return labels;
}

function CitationView({ node, editor, selected }: ReactNodeViewProps) {
  const labels = useEditorState({
    editor,
    selector: ({ editor }) => Array.from(citationLabels(editor).entries()),
    equalityFn: (a, b) => JSON.stringify(a) === JSON.stringify(b),
  });
  const map = new Map(labels);
  const keys: string[] = node.attrs.keys;
  const authorYear = editor.storage.texloom.template?.bibliography.style === "authorYear";
  const missing = keys.filter((k) => !map.has(k));
  const parts = keys.map((k) => map.get(k) ?? "?");
  const text = authorYear ? `(${parts.join("; ")})` : `[${parts.join(", ")}]`;
  const entries = editor.storage.texloom.bibEntries;
  const title = keys
    .map((k) => {
      const e = entries.find((x) => x.key === k);
      return e ? `${k}: ${e.authors.join(", ")} (${e.year ?? "n.d."}) ${e.title}` : `${k}: not in the references`;
    })
    .join("\n");
  return (
    <NodeViewWrapper as="span" className={`citation${missing.length ? " broken" : ""}${selected ? " is-selected" : ""}`} title={title}>
      {text}
    </NodeViewWrapper>
  );
}

export const Citation = Node.create({
  name: "citation",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return {
      keys: {
        default: [],
        parseHTML: (el) => (el.getAttribute("data-keys") ?? "").split(",").filter(Boolean),
        renderHTML: (attrs) => ({ "data-keys": (attrs.keys as string[]).join(",") }),
      },
    };
  },
  parseHTML: () => [{ tag: "span[data-keys]" }],
  renderHTML: ({ HTMLAttributes }) => ["span", mergeAttributes(HTMLAttributes, { class: "citation" })],
  addNodeView() {
    return ReactNodeViewRenderer(CitationView, { as: "span" });
  },
  addCommands() {
    return {
      // Right after an existing citation, add the key to it: "[1, 2]" instead of "[1] [2]".
      insertCitation:
        (key: string) =>
        ({ state, chain }) => {
          const { $from, empty } = state.selection;
          const before = empty ? $from.nodeBefore : null;
          if (before?.type.name === this.name) {
            const keys: string[] = before.attrs.keys;
            if (keys.includes(key)) return true;
            const pos = $from.pos - before.nodeSize;
            return chain()
              .command(({ tr }) => {
                tr.setNodeMarkup(pos, undefined, { keys: [...keys, key] });
                return true;
              })
              .run();
          }
          return chain().insertContent({ type: this.name, attrs: { keys: [key] } }).run();
        },
    };
  },
});

// ---------------------------------------------------------------- appendices

export const AppendixStart = Node.create({
  name: "appendixStart",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,

  parseHTML: () => [{ tag: "div[data-appendix-start]" }],
  renderHTML: ({ HTMLAttributes }) => [
    "div",
    mergeAttributes(HTMLAttributes, { "data-appendix-start": "", class: "appendix-divider", contenteditable: "false" }),
    "Appendices: chapters below are lettered A, B…",
  ],

  addCommands() {
    return {
      insertAppendices:
        () =>
        ({ state, commands }) => {
          let exists = false;
          state.doc.forEach((n) => void (n.type.name === this.name && (exists = true)));
          if (exists) return false;
          return commands.insertContent([
            { type: this.name },
            { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Appendix title" }] },
          ]);
        },
    };
  },
});
