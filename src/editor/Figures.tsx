// Figures (image + caption below) and captioned tables (caption above + table).
// Numbering ("Figure 3.1") comes from CSS counters set up by the template styles.
import { useEffect, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
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
export function useDocumentImage(src: string) {
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

/** Percent cut off at each edge of an image. */
export interface Crop {
  left: number;
  right: number;
  top: number;
  bottom: number;
}
const NO_CROP: Crop = { left: 0, right: 0, top: 0, bottom: 0 };
/** Most of an edge that can be cut (the LaTeX side keeps the same limit). */
const MAX_CROP = 45;

/** Styles that show only the uncropped part of an image: the image is made larger
 *  and shifted inside a box that hides the rest. Margins in percent are of the box's
 *  width, so the vertical ones need the image's proportions. */
function croppedImageStyle(c: Crop, aspect: number): React.CSSProperties {
  const visible = Math.max(0.1, 1 - (c.left + c.right) / 100);
  const w = 100 / visible; // image width, % of the box
  return {
    width: `${w}%`,
    maxWidth: "none",
    marginLeft: `${(-c.left / 100) * w}%`,
    marginTop: `${(-c.top / 100) * w * aspect}%`,
    marginBottom: `${(-c.bottom / 100) * w * aspect}%`,
  };
}

/** Crop mode: the whole image with a frame whose edges are dragged. */
function CropFrame({ url, crop, onChange }: { url: string; crop: Crop; onChange: (c: Crop) => void }) {
  const box = useRef<HTMLDivElement>(null);
  const drag = (side: keyof Crop) => (e: ReactPointerEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const el = box.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    const start = { ...crop };
    const move = (ev: PointerEvent) => {
      const fx = ((ev.clientX - rect.left) / rect.width) * 100;
      const fy = ((ev.clientY - rect.top) / rect.height) * 100;
      const v = { left: fx, right: 100 - fx, top: fy, bottom: 100 - fy }[side];
      // Each edge stays within its limit and leaves at least 10% of the image.
      const opposite = { left: start.right, right: start.left, top: start.bottom, bottom: start.top }[side];
      const clamped = Math.round(Math.min(MAX_CROP, 90 - opposite, Math.max(0, v)) * 10) / 10;
      onChange({ ...start, [side]: clamped });
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  };
  return (
    <div className="crop-area" ref={box}>
      <img src={url} alt="" draggable={false} />
      <div className="crop-frame" style={{ left: `${crop.left}%`, right: `${crop.right}%`, top: `${crop.top}%`, bottom: `${crop.bottom}%` }}>
        {(["left", "right", "top", "bottom"] as const).map((side) => (
          <span key={side} className={`crop-handle crop-${side}`} onPointerDown={drag(side)} title="Drag to crop" />
        ))}
      </div>
    </div>
  );
}

function FigureView({ node, updateAttributes, selected }: ReactNodeViewProps) {
  const src: string = node.attrs.src;
  const width: number = node.attrs.widthPercent;
  const crop: Crop = { ...NO_CROP, ...(node.attrs.crop ?? {}) };
  const cropped = crop.left + crop.right + crop.top + crop.bottom > 0;
  const { url, error } = useDocumentImage(src);
  const [problem, setProblem] = useState<string | null>(null);
  /** The image's height / width, for showing a crop. */
  const [aspect, setAspect] = useState(0.75);
  /** Crop mode's frame (null when not cropping). */
  const [cropDraft, setCropDraft] = useState<Crop | null>(null);

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
      <div className={`figure-image${cropped && !cropDraft ? " cropped" : ""}`} contentEditable={false} style={{ width: `${width}%` }}>
        {url && cropDraft ? (
          <CropFrame url={url} crop={cropDraft} onChange={setCropDraft} />
        ) : url ? (
          <img
            src={url}
            alt=""
            draggable={false}
            onLoad={(e) => e.currentTarget.naturalWidth && setAspect(e.currentTarget.naturalHeight / e.currentTarget.naturalWidth)}
            style={cropped ? croppedImageStyle(crop, aspect) : undefined}
          />
        ) : (
          <div className="figure-placeholder">{error ?? (src ? `${src} (PDF image: shown in the preview)` : "No image")}</div>
        )}
      </div>
      <div className={`figure-controls${cropDraft ? " cropping" : ""}`} contentEditable={false}>
        {cropDraft ? (
          <>
            <span>Drag the edges to crop</span>
            <button type="button" onClick={() => setCropDraft({ ...NO_CROP })}>
              Reset
            </button>
            <button type="button" onClick={() => setCropDraft(null)}>
              Cancel
            </button>
            <button
              type="button"
              className="primary"
              onClick={() => {
                const c = cropDraft;
                updateAttributes({ crop: c.left + c.right + c.top + c.bottom > 0 ? c : null });
                setCropDraft(null);
              }}
            >
              Done
            </button>
          </>
        ) : (
          <>
            <label>
              Width
              <input type="range" min={10} max={100} step={5} value={width} onChange={(e) => updateAttributes({ widthPercent: Number(e.target.value) })} />
              <span>{width}%</span>
            </label>
            <button type="button" onClick={() => setCropDraft({ ...crop })} disabled={!url} title={url ? "Cut off parts of the image at its edges" : "PDF images can't be cropped here"}>
              Crop…
            </button>
            <button type="button" onClick={replace}>
              Replace image…
            </button>
          </>
        )}
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
      // Percent cut off at each edge ({left, right, top, bottom}); null for none.
      crop: { default: null, rendered: false },
    };
  },

  parseHTML: () => [{ tag: "figure[data-figure]" }],
  renderHTML: ({ HTMLAttributes }) => ["figure", mergeAttributes(HTMLAttributes, { "data-figure": "" }), 0],

  addNodeView() {
    // Let the width slider and buttons work without ProseMirror grabbing the events.
    return ReactNodeViewRenderer(FigureView, {
      stopEvent: ({ event }) => !!(event.target as HTMLElement).closest?.(".figure-controls, .crop-area"),
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
