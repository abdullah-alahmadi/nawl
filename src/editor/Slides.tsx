// Presentation nodes: slides (a title line, then content), columns, titled boxes
// (beamer blocks) and pauses. The document of a presentation is a list of slides
// with section headings between them, shown down the page like any document.
import { Node, mergeAttributes, NodeViewContent, NodeViewWrapper, ReactNodeViewRenderer, type ReactNodeViewProps } from "@tiptap/react";
import Document from "@tiptap/extension-document";
import { TextSelection, type EditorState } from "@tiptap/pm/state";

declare module "@tiptap/react" {
  interface Commands<ReturnType> {
    slides: {
      /** A new slide after the current one (or after the current section heading). */
      insertSlide: () => ReturnType;
      /** A section heading after the current slide. */
      insertSection: () => ReturnType;
      insertColumns: (count?: number) => ReturnType;
      insertSlideBox: (kind?: SlideBoxKind) => ReturnType;
      insertPause: () => ReturnType;
      /** In a slide's title: moves to an empty line below it, so a block can go there. */
      leaveSlideTitle: () => ReturnType;
    };
  }
}

export type SlideBoxKind = "block" | "alert" | "example";

/** Documents may hold slides at the top level, next to ordinary blocks. */
export const DeckDocument = Document.extend({ content: "(block | slide)+" });

export const SlideTitle = Node.create({
  name: "slideTitle",
  content: "inline*",
  marks: "_",
  defining: true,
  parseHTML: () => [{ tag: "div[data-slide-title]" }],
  renderHTML: ({ HTMLAttributes }) => ["div", mergeAttributes(HTMLAttributes, { "data-slide-title": "", class: "slide-title" }), 0],
  addKeyboardShortcuts() {
    return {
      // Enter in the title goes to the slide's content (a new first line if needed).
      Enter: ({ editor }) => {
        const { $from } = editor.state.selection;
        if ($from.parent.type.name !== "slideTitle") return false;
        const after = $from.after();
        const next = editor.state.doc.resolve(after).nodeAfter;
        if (next?.type.name === "paragraph" && next.content.size === 0) return editor.commands.setTextSelection(after + 1);
        return editor.chain().insertContentAt(after, { type: "paragraph" }).setTextSelection(after + 1).run();
      },
    };
  },
});

export const Slide = Node.create({
  name: "slide",
  group: "slide",
  content: "slideTitle block*",
  defining: true,
  isolating: true,
  addAttributes() {
    return {
      id: { default: null },
      plain: {
        default: false,
        parseHTML: (el) => el.hasAttribute("data-plain"),
        renderHTML: (a) => (a.plain ? { "data-plain": "" } : {}),
      },
    };
  },
  parseHTML: () => [{ tag: "section[data-slide]" }],
  renderHTML: ({ HTMLAttributes }) => ["section", mergeAttributes(HTMLAttributes, { "data-slide": "", class: "slide" }), 0],

  addCommands() {
    return {
      leaveSlideTitle:
        () =>
        ({ state, dispatch }) => {
          const { $from } = state.selection;
          if ($from.parent.type.name !== "slideTitle") return true;
          const after = $from.after();
          const next = state.doc.resolve(after).nodeAfter;
          if (dispatch) {
            const tr = state.tr;
            if (!(next?.type.name === "paragraph" && next.content.size === 0)) tr.insert(after, state.schema.nodes.paragraph.create());
            tr.setSelection(TextSelection.create(tr.doc, after + 1));
            dispatch(tr);
          }
          return true;
        },
      insertSlide:
        () =>
        ({ state, dispatch }) => {
          const at = afterCurrentTop(state);
          const { schema } = state;
          const slide = schema.nodes.slide.create(null, [schema.nodes.slideTitle.create(), schema.nodes.paragraph.create()]);
          if (dispatch) {
            const tr = state.tr.insert(at, slide);
            tr.setSelection(TextSelection.create(tr.doc, at + 2)).scrollIntoView();
            dispatch(tr);
          }
          return true;
        },
      insertSection:
        () =>
        ({ state, dispatch }) => {
          const at = afterCurrentTop(state);
          const heading = state.schema.nodes.heading.create({ level: 1 });
          if (dispatch) {
            const tr = state.tr.insert(at, heading);
            tr.setSelection(TextSelection.create(tr.doc, at + 1)).scrollIntoView();
            dispatch(tr);
          }
          return true;
        },
      insertColumns:
        (count = 2) =>
        ({ state, dispatch }) => {
          const { schema } = state;
          const columns = schema.nodes.columns.create(
            null,
            Array.from({ length: count }, () =>
              schema.nodes.column.create({ widthPercent: Math.round(100 / count) }, schema.nodes.paragraph.create()),
            ),
          );
          if (dispatch) {
            // An empty line becomes the columns; otherwise they go below the current block.
            const { $from } = state.selection;
            const tr = state.tr;
            let at: number;
            if ($from.parent.type.name === "paragraph" && $from.parent.content.size === 0) {
              at = $from.before();
              tr.replaceWith(at, $from.after(), columns);
            } else {
              at = $from.depth > 0 ? $from.after() : state.doc.content.size;
              tr.insert(at, columns);
            }
            // The cursor goes into the first column: columns > column > paragraph.
            tr.setSelection(TextSelection.create(tr.doc, at + 3)).scrollIntoView();
            dispatch(tr);
          }
          return true;
        },
      insertSlideBox:
        (kind = "block") =>
        ({ commands }) =>
          commands.insertContent({ type: "slideBox", attrs: { kind, title: "" }, content: [{ type: "paragraph" }] }),
      insertPause:
        () =>
        ({ commands }) =>
          commands.insertContent([{ type: "slidePause" }, { type: "paragraph" }]),
    };
  },
});

/** Position just after the top-level node holding the cursor. */
function afterCurrentTop(state: EditorState): number {
  const { $from } = state.selection;
  if ($from.depth === 0) return state.doc.content.size;
  return $from.after(1);
}

export const Columns = Node.create({
  name: "columns",
  group: "block",
  content: "column{2,4}",
  defining: true,
  isolating: true,
  parseHTML: () => [{ tag: "div[data-columns]" }],
  renderHTML: ({ HTMLAttributes }) => ["div", mergeAttributes(HTMLAttributes, { "data-columns": "", class: "slide-columns" }), 0],
});

export const Column = Node.create({
  name: "column",
  content: "block+",
  isolating: true,
  addAttributes() {
    return {
      widthPercent: {
        default: 50,
        parseHTML: (el) => Number(el.getAttribute("data-width")) || 50,
        renderHTML: (a) => ({ "data-width": a.widthPercent, style: `flex: 0 0 calc(${a.widthPercent}% - 8px)` }),
      },
    };
  },
  parseHTML: () => [{ tag: "div[data-column]" }],
  renderHTML: ({ HTMLAttributes }) => ["div", mergeAttributes(HTMLAttributes, { "data-column": "", class: "slide-column" }), 0],
});

const BOX_LABELS: Record<SlideBoxKind, string> = { block: "Block", alert: "Alert", example: "Example" };

function SlideBoxView({ node, updateAttributes }: ReactNodeViewProps) {
  const kind = (node.attrs.kind ?? "block") as SlideBoxKind;
  return (
    <NodeViewWrapper className={`slide-box slide-box-${kind}`} data-slide-box="">
      <div className="slide-box-title" contentEditable={false}>
        <input
          value={node.attrs.title ?? ""}
          placeholder={`${BOX_LABELS[kind]} title`}
          onChange={(e) => updateAttributes({ title: e.target.value })}
          aria-label="Box title"
          spellCheck
        />
        <select value={kind} onChange={(e) => updateAttributes({ kind: e.target.value })} aria-label="Box style" title="Box style">
          {(Object.keys(BOX_LABELS) as SlideBoxKind[]).map((k) => (
            <option key={k} value={k}>
              {BOX_LABELS[k]}
            </option>
          ))}
        </select>
      </div>
      <NodeViewContent className="slide-box-body" />
    </NodeViewWrapper>
  );
}

export const SlideBox = Node.create({
  name: "slideBox",
  group: "block",
  content: "block+",
  defining: true,
  addAttributes() {
    return { kind: { default: "block" }, title: { default: "" } };
  },
  parseHTML: () => [{ tag: "div[data-slide-box]" }],
  renderHTML: ({ HTMLAttributes }) => ["div", mergeAttributes(HTMLAttributes, { "data-slide-box": "" }), 0],
  addNodeView() {
    return ReactNodeViewRenderer(SlideBoxView);
  },
});

export const SlidePause = Node.create({
  name: "slidePause",
  group: "block",
  atom: true,
  selectable: true,
  parseHTML: () => [{ tag: "div[data-slide-pause]" }],
  renderHTML: ({ HTMLAttributes }) => [
    "div",
    mergeAttributes(HTMLAttributes, { "data-slide-pause": "", class: "slide-pause", contenteditable: "false" }),
    ["span", {}, "Pause: the rest appears on the next click"],
  ],
});

export const SLIDE_EXTENSIONS = [SlideTitle, Slide, Columns, Column, SlideBox, SlidePause];
