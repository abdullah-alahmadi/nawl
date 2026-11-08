// Cross-references, citations, appendices — and the stable ids that references
// point at. On-screen numbers come from numbering.ts; the PDF uses \ref and BibTeX.
import { Extension, Node, mergeAttributes, useEditorState } from "@tiptap/react";
import type { Editor } from "@tiptap/react";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { Node as PMNode } from "@tiptap/pm/model";
import type { NodeView } from "@tiptap/pm/view";
import type { Template } from "../api";
import type { BibEntry } from "../api";
import { docLanguage, hasArabic, localDigits, words } from "./language";
import { computeNumbering, type Counters, type NumberedItem } from "./numbering";
import { t as tr } from "../i18n";

/** Node types that cross-references can point at. */
const TARGET_TYPES = ["heading", "figure", "tableFigure", "equation"];

const newId = () => Math.random().toString(36).slice(2, 10);

/** What node views need from the app; changed with `setTexloomContext`. */
export interface TexloomContext {
  template: Template | null;
  bibEntries: BibEntry[];
  /** The open document is a chapter of a project. */
  project: ProjectContext | null;
}

/** For a chapter of a project: what the other chapters contribute (from the index). */
export interface ProjectContext {
  /** Numbering starts where the chapters before this one end. */
  start: Counters;
  /** The other chapters' numbered items, for references to them. */
  others: NumberedItem[];
  /** Citation keys of the chapters before and after this one, in order (citations
   *  are numbered in book order). */
  citationsBefore: string[];
  citationsAfter: string[];
  /** Opens another chapter at a target (Ctrl-click on a reference to it). */
  openTarget: (chapter: string, id: string) => void;
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
      /** Numbers the heading at the cursor, or leaves it unnumbered ("Acknowledgment"). */
      toggleHeadingNumbered: () => ReturnType;
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
    return { template: null, bibEntries: [], project: null };
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
      {
        types: ["heading"],
        attributes: {
          numbered: {
            default: true,
            parseHTML: (el) => !el.hasAttribute("data-unnumbered"),
            renderHTML: (attrs) => (attrs.numbered === false ? { "data-unnumbered": "" } : {}),
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      toggleHeadingNumbered:
        () =>
        ({ state, commands }) => {
          const { $from } = state.selection;
          for (let d = $from.depth; d > 0; d--) {
            const node = $from.node(d);
            if (node.type.name === "heading") {
              return commands.updateAttributes("heading", { numbered: node.attrs.numbered === false });
            }
          }
          return false;
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey("texloom-ids"),
        appendTransaction: (transactions, _old, state) => {
          if (!transactions.some((tr) => tr.docChanged) && _old.doc.eq(state.doc)) return null;
          // In a project, ids are unique across chapters too: a figure copied from
          // another chapter gets a new one.
          const seen = new Set<string>(this.storage.project?.others.flatMap((i: NumberedItem) => (i.id ? [i.id] : [])) ?? []);
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

type Numbering = { template: unknown; project: unknown; items: NumberedItem[]; byId: Map<string, NumberedItem> };
const NO_NUMBERING: Numbering = { template: null, project: null, items: [], byId: new Map() };
const numberingCache = new WeakMap<PMNode, Numbering>();

/** The document's numbered targets, computed once per version of the document
 *  (documents are immutable) and shared by every node view that shows a number:
 *  computed per view, typing in a long book redid it hundreds of times a keystroke.
 *  In a project, `byId` also has the other chapters' targets. */
function numberingOf(editor: Editor): Numbering {
  const { template, project } = editor.storage.texloom;
  if (!template) return NO_NUMBERING;
  const doc = editor.state.doc;
  let cached = numberingCache.get(doc);
  if (!cached || cached.template !== template || cached.project !== project) {
    const items = computeNumbering(doc, template, project?.start);
    const byId = new Map(items.flatMap((i) => (i.id ? [[i.id, i] as const] : [])));
    for (const i of project?.others ?? []) if (i.id && !byId.has(i.id)) byId.set(i.id, i);
    cached = { template, project, items, byId };
    numberingCache.set(doc, cached);
  }
  return cached;
}

type LabelStore = { version: number; signature: string; listeners: Set<() => void> };
const labelStores = new WeakMap<Editor, LabelStore>();

/** One listener per editor for every reference and citation on the page: after each
 *  transaction it compares the labels they show (numbers, captions, citation labels)
 *  and wakes the views only when one changed. A listener per view (thousands in a
 *  long book) made every keystroke slow, though typing rarely changes a label. */
function labelStore(editor: Editor): LabelStore {
  let store = labelStores.get(editor);
  if (store) return store;
  const s: LabelStore = { version: 0, signature: "", listeners: new Set() };
  const check = () => {
    const signature = JSON.stringify([[...numberingOf(editor).byId.values()].map((i) => [i.id, i.refText, i.text]), [...citationLabels(editor)]]);
    if (signature === s.signature) return;
    s.signature = signature;
    s.version++;
    s.listeners.forEach((l) => l());
  };
  editor.on("transaction", check);
  check();
  labelStores.set(editor, s);
  store = s;
  return store;
}

/**
 * A plain (non-React) node view for an inline label (a reference or a citation),
 * redrawn when its node or any label changes. A long book has thousands of these, and
 * as React node views they took seconds to mount.
 */
class LabelView implements NodeView {
  dom: HTMLSpanElement;
  private unsubscribe: () => void;

  constructor(
    public node: PMNode,
    private editor: Editor,
    private draw: (dom: HTMLSpanElement, node: PMNode, editor: Editor) => void,
    private tooltip: (node: PMNode, editor: Editor) => string,
  ) {
    this.dom = document.createElement("span");
    // Tooltips are built when hovered, not for every label up front.
    this.dom.addEventListener("mouseenter", () => {
      this.dom.title = this.tooltip(this.node, this.editor);
    });
    const store = labelStore(editor);
    const redraw = () => this.draw(this.dom, this.node, this.editor);
    store.listeners.add(redraw);
    this.unsubscribe = () => void store.listeners.delete(redraw);
    redraw();
  }

  update(node: PMNode) {
    if (node.type !== this.node.type) return false;
    if (node.attrs !== this.node.attrs) {
      this.node = node;
      this.draw(this.dom, node, this.editor);
    }
    this.node = node;
    return true;
  }

  selectNode() {
    this.dom.classList.add("is-selected");
  }

  deselectNode() {
    this.dom.classList.remove("is-selected");
  }

  ignoreMutation() {
    return true;
  }

  destroy() {
    this.unsubscribe();
  }
}

/** Sets the label's text and classes, keeping the selection highlight. */
function setLabel(dom: HTMLSpanElement, base: string, broken: boolean, text: string) {
  const selected = dom.classList.contains("is-selected");
  dom.className = `${base}${broken ? " broken" : ""}${selected ? " is-selected" : ""}`;
  if (dom.textContent !== text) dom.textContent = text;
}

/** Numbered targets of the current document (empty until a template is set). */
export function useNumbering(editor: Editor): NumberedItem[] {
  return useEditorState({
    editor,
    selector: ({ editor }) => numberingOf(editor).items,
    equalityFn: (a, b) => a === b || JSON.stringify(a) === JSON.stringify(b),
  });
}

/** What a cross-reference can point at: this document's numbered targets, and in a
 *  project the other chapters' (marked with their `chapter`). */
export function useReferenceTargets(editor: Editor): NumberedItem[] {
  return useEditorState({
    editor,
    selector: ({ editor }) => {
      const own = numberingOf(editor).items;
      const others = editor.storage.texloom.project?.others ?? [];
      return others.length ? [...own, ...others] : own;
    },
    equalityFn: (a, b) => a === b || JSON.stringify(a) === JSON.stringify(b),
  });
}

// ---------------------------------------------------------------- cross-references

function refView(node: PMNode, editor: Editor): NodeView {
  const view = new LabelView(
    node,
    editor,
    (dom, node, editor) => {
      const shown = numberingOf(editor).byId.get(node.attrs.target);
      setLabel(dom, "ref", !shown, shown ? shown.refText : "??");
    },
    (node, editor) => {
      const shown = numberingOf(editor).byId.get(node.attrs.target);
      return shown ? tr("editor.refTooltip", { ref: shown.refText, text: shown.text }) : tr("editor.refBroken");
    },
  );
  // Ctrl/Cmd-click jumps to what the reference points at.
  view.dom.addEventListener("click", (e) => {
    const target = numberingOf(editor).byId.get(view.node.attrs.target);
    if (!target || !(e.ctrlKey || e.metaKey)) return;
    e.preventDefault();
    if (target.chapter && target.id) editor.storage.texloom.project?.openTarget(target.chapter, target.id);
    else editor.chain().focus().setTextSelection(target.pos + 1).scrollIntoView().run();
  });
  return view;
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
    return ({ node, editor }) => refView(node, editor);
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

const citationCache = new WeakMap<PMNode, { template: unknown; bibEntries: unknown; project: unknown; labels: Map<string, string> }>();

/** On-screen citation labels, matching the template's bibliography style: computed
 *  once per version of the document and shared by every citation on the page. */
export function citationLabels(editor: Editor): Map<string, string> {
  const { template, bibEntries, project } = editor.storage.texloom;
  const doc = editor.state.doc;
  const cached = citationCache.get(doc);
  if (cached && cached.template === template && cached.bibEntries === bibEntries && cached.project === project) return cached.labels;
  const labels = computeCitationLabels(editor);
  citationCache.set(doc, { template, bibEntries, project, labels });
  return labels;
}

function computeCitationLabels(editor: Editor): Map<string, string> {
  const { template, bibEntries } = editor.storage.texloom;
  const labels = new Map<string, string>();
  if (!template) return labels;
  const byKey = new Map(bibEntries.map((e) => [e.key, e]));
  // Keys in the order they're first cited (in a project: in the whole book).
  const project = editor.storage.texloom.project;
  const seen = new Set<string>(project?.citationsBefore ?? []);
  editor.state.doc.descendants((n) => {
    if (n.type.name === "citation") for (const k of n.attrs.keys as string[]) seen.add(k);
  });
  for (const k of project?.citationsAfter ?? []) seen.add(k);
  const order = [...seen];
  const style = template.bibliography.style;
  const digits = docLanguage(template).digits;
  if (style === "authorYear") {
    for (const k of order) {
      const e = byKey.get(k);
      if (!e) continue;
      const a = e.authors;
      // The entry's own language: "et al." / "وآخرون", as in the reference list.
      const w = words(hasArabic(e.title + a.join(" ")) ? "arabic" : "english");
      const and = w.and === "و" ? ` ${w.and}` : ` ${w.and} `;
      const who = a.length === 0 ? e.title || k : a.length === 1 ? a[0] : a.length === 2 ? `${a[0]}${and}${a[1]}` : `${a[0]} ${w.etAl}`;
      labels.set(k, `${who}, ${localDigits(e.year ?? w.noDate, digits)}`);
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
    known.forEach((k, i) => labels.set(k, localDigits(String(i + 1), digits)));
  }
  return labels;
}

function drawCitation(dom: HTMLSpanElement, node: PMNode, editor: Editor) {
  const keys: string[] = node.attrs.keys;
  const all = citationLabels(editor);
  const own = keys.map((k) => all.get(k) ?? null);
  const authorYear = editor.storage.texloom.template?.bibliography.style === "authorYear";
  const parts = own.map((l) => l ?? "?");
  const text = authorYear ? `(${parts.join("; ")})` : `[${parts.join(", ")}]`;
  setLabel(dom, "citation", own.includes(null), text);
}

function citationTooltip(node: PMNode, editor: Editor): string {
  const entries = editor.storage.texloom.bibEntries;
  return (node.attrs.keys as string[])
    .map((k) => {
      const e = entries.find((x) => x.key === k);
      return e ? `${k}: ${e.authors.join(", ")} (${e.year ?? tr("editor.citationNoDate")}) ${e.title}` : tr("editor.citationMissing", { key: k });
    })
    .join("\n");
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
    return ({ node, editor }) => new LabelView(node, editor, drawCitation, citationTooltip);
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
    // Interface text, not document content: the PDF has no such line.
    tr("editor.appendixDivider"),
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
