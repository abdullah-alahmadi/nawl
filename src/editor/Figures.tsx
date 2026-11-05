// Figures (image + caption below) and captioned tables (caption above + table).
// Numbering ("Figure 3.1") comes from CSS counters set up by the template styles.
import { useEffect, useState } from "react";
import {
  Node,
  mergeAttributes,
  NodeViewContent,
  NodeViewWrapper,
  ReactNodeViewRenderer,
  type Editor,
  type ReactNodeViewProps,
} from "@tiptap/react";
import { TextSelection } from "@tiptap/pm/state";
import * as api from "../api";

declare module "@tiptap/react" {
  interface Commands<ReturnType> {
    figures: {
      insertFigure: (src: string) => ReturnType;
      insertCaptionedTable: (rows?: number, cols?: number) => ReturnType;
      deleteCaptionedTable: () => ReturnType;
    };
  }
}

/** Caption text inside a figure or captioned table. Enter leaves the caption. */
export const Caption = Node.create({
  name: "caption",
  content: "inline*",
  marks: "_",
  defining: true,
  parseHTML: () => [{ tag: "figcaption" }],
  renderHTML: ({ HTMLAttributes }) => ["figcaption", mergeAttributes(HTMLAttributes, { class: "caption" }), 0],
  addKeyboardShortcuts() {
    return {
      Enter: ({ editor }) => exitCaption(editor),
    };
  },
});

/** Moves the cursor to a new paragraph after the figure/table the caption belongs to. */
function exitCaption(editor: Editor): boolean {
  const { $from } = editor.state.selection;
  if ($from.parent.type.name !== "caption") return false;
  for (let d = $from.depth - 1; d > 0; d--) {
    const node = $from.node(d);
    if (node.type.name === "figure" || node.type.name === "tableFigure") {
      const after = $from.after(d);
      return editor
        .chain()
        .insertContentAt(after, { type: "paragraph" })
        .command(({ tr }) => {
          tr.setSelection(TextSelection.create(tr.doc, after + 1));
          return true;
        })
        .scrollIntoView()
        .run();
    }
  }
  return false;
}

/** Object URL for an image in the document's folder. */
function useDocumentImage(src: string) {
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!src) return;
    let made: string | null = null;
    let cancelled = false;
    setError(null);
    if (src.toLowerCase().endsWith(".pdf")) {
      setUrl(null);
      return;
    }
    api.readDocumentFile(src).then(
      (bytes) => {
        if (cancelled) return;
        made = URL.createObjectURL(new Blob([bytes]));
        setUrl(made);
      },
      (e) => !cancelled && setError(api.asAppError(e).kind === "other" ? (api.asAppError(e) as { message: string }).message : "Can't load image"),
    );
    return () => {
      cancelled = true;
      if (made) URL.revokeObjectURL(made);
    };
  }, [src]);
  return { url, error };
}

/** Asks for an image file and copies it into the document's figures/ folder. */
export async function pickFigureImage(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const file = await open({ multiple: false, directory: false, filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "pdf"] }] });
  if (!file) return null;
  return api.importFigure(file);
}

function FigureView({ node, updateAttributes, selected }: ReactNodeViewProps) {
  const src: string = node.attrs.src;
  const width: number = node.attrs.widthPercent;
  const { url, error } = useDocumentImage(src);
  const [problem, setProblem] = useState<string | null>(null);

  const replace = async () => {
    try {
      const next = await pickFigureImage();
      if (next) updateAttributes({ src: next });
    } catch (e) {
      setProblem(String((api.asAppError(e) as { message?: string }).message ?? e));
    }
  };

  return (
    <NodeViewWrapper as="figure" className={`figure${selected ? " is-selected" : ""}`} data-figure="">
      <div className="figure-image" contentEditable={false} style={{ width: `${width}%` }}>
        {url ? (
          <img src={url} alt="" draggable={false} />
        ) : (
          <div className="figure-placeholder">{error ?? (src ? `${src} (PDF image: shown in the preview)` : "No image")}</div>
        )}
      </div>
      <div className="figure-controls" contentEditable={false}>
        <label>
          Width
          <input type="range" min={10} max={100} step={5} value={width} onChange={(e) => updateAttributes({ widthPercent: Number(e.target.value) })} />
          <span>{width}%</span>
        </label>
        <button type="button" onClick={replace}>
          Replace image…
        </button>
        {problem && <span className="figure-problem">{problem}</span>}
      </div>
      <NodeViewContent className="figure-caption" />
    </NodeViewWrapper>
  );
}

export const Figure = Node.create({
  name: "figure",
  group: "block",
  content: "caption",
  draggable: true,
  isolating: true,

  addAttributes() {
    return {
      src: { default: "" },
      widthPercent: { default: 80 },
    };
  },

  parseHTML: () => [{ tag: "figure[data-figure]" }],
  renderHTML: ({ HTMLAttributes }) => ["figure", mergeAttributes(HTMLAttributes, { "data-figure": "" }), 0],

  addNodeView() {
    // Let the width slider and buttons work without ProseMirror grabbing the events.
    return ReactNodeViewRenderer(FigureView, {
      stopEvent: ({ event }) => !!(event.target as HTMLElement).closest?.(".figure-controls"),
    });
  },

  addCommands() {
    return {
      insertFigure:
        (src: string) =>
        ({ chain }) =>
          chain()
            .insertContent({ type: this.name, attrs: { src }, content: [{ type: "caption" }] })
            .command(({ tr }) => {
              // Put the cursor in the new caption: search back from the cursor for it.
              let pos: number | null = null;
              tr.doc.nodesBetween(Math.max(0, tr.selection.from - 4), tr.selection.from, (n, p) => {
                if (n.type.name === "caption") pos = p + 1;
              });
              if (pos !== null) tr.setSelection(TextSelection.create(tr.doc, pos));
              return true;
            })
            .run(),
    };
  },
});

/** A table with its caption above it, numbered "Table 3.1". */
export const CaptionedTable = Node.create({
  name: "tableFigure",
  group: "block",
  content: "caption table",
  isolating: true,

  parseHTML: () => [{ tag: "div[data-table-figure]" }],
  renderHTML: ({ HTMLAttributes }) => ["div", mergeAttributes(HTMLAttributes, { "data-table-figure": "", class: "table-figure" }), 0],

  addCommands() {
    return {
      insertCaptionedTable:
        (rows = 3, cols = 3) =>
        ({ commands }) => {
          const cell = (header: boolean) => ({ type: header ? "tableHeader" : "tableCell", content: [{ type: "paragraph" }] });
          const row = (header: boolean) => ({ type: "tableRow", content: Array.from({ length: cols }, () => cell(header)) });
          return commands.insertContent({
            type: this.name,
            content: [
              { type: "caption" },
              { type: "table", content: [row(true), ...Array.from({ length: rows - 1 }, () => row(false))] },
            ],
          });
        },
      deleteCaptionedTable:
        () =>
        ({ state, dispatch }) => {
          const { $from } = state.selection;
          for (let d = $from.depth; d > 0; d--) {
            if ($from.node(d).type.name === "tableFigure" || ($from.node(d).type.name === "table" && $from.node(d - 1)?.type.name !== "tableFigure")) {
              if (dispatch) dispatch(state.tr.delete($from.before(d), $from.after(d)));
              return true;
            }
          }
          return false;
        },
    };
  },
});
