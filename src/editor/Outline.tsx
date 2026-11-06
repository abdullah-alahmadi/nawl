// Outline of the document's headings, figures and tables; click to jump there.
// A presentation lists its sections and numbered slides instead.
import { useEditorState, type Editor } from "@tiptap/react";
import { TextSelection } from "@tiptap/pm/state";
import { useNumbering } from "./Academic";

interface SlideEntry {
  kind: "section" | "slide";
  text: string;
  pos: number;
  number: number;
}

/** Sections and slides of a presentation, in order (empty for other documents). */
function slideEntries(editor: Editor): SlideEntry[] {
  const out: SlideEntry[] = [];
  let n = 0;
  editor.state.doc.forEach((node, pos) => {
    if (node.type.name === "slide") out.push({ kind: "slide", text: node.firstChild?.textContent.trim() || "Untitled slide", pos, number: ++n });
    else if (node.type.name === "heading") out.push({ kind: "section", text: node.textContent.trim() || "Untitled section", pos, number: 0 });
  });
  return n > 0 ? out : [];
}

export function Outline({ editor }: { editor: Editor }) {
  const entries = useNumbering(editor).filter((e) => e.kind !== "equation");
  const slides =
    useEditorState({
      editor,
      selector: ({ editor }) => slideEntries(editor),
      equalityFn: (a, b) => JSON.stringify(a) === JSON.stringify(b),
    }) ?? [];

  const jump = (pos: number) => {
    const { state, view } = editor;
    const target = Math.min(pos + 1, state.doc.content.size);
    view.dispatch(state.tr.setSelection(TextSelection.near(state.doc.resolve(target))));
    view.focus();
    const dom = view.nodeDOM(pos);
    if (dom instanceof HTMLElement) dom.scrollIntoView({ block: "start", behavior: "smooth" });
  };

  if (slides.length > 0)
    return (
      <nav className="outline" aria-label="Presentation outline">
        <div className="outline-title">Slides</div>
        {slides.map((e) => (
          <button
            key={e.pos}
            className={`outline-item ${e.kind === "section" ? "outline-heading" : "outline-slide"}`}
            style={{ paddingLeft: e.kind === "section" ? 8 : 20 }}
            onClick={() => jump(e.pos + (e.kind === "slide" ? 1 : 0))}
            title={e.text}
          >
            {e.kind === "slide" && <span className="outline-number">{e.number}</span>}
            <span className="outline-text">{e.text}</span>
          </button>
        ))}
      </nav>
    );

  return (
    <nav className="outline" aria-label="Document outline">
      <div className="outline-title">Outline</div>
      {entries.length === 0 && <p className="muted outline-empty">Headings, figures and tables appear here.</p>}
      {entries.map((e, i) => (
        <button
          key={i}
          className={`outline-item outline-${e.kind}${e.appendix && e.kind === "heading" && e.level === 1 ? " outline-appendix" : ""}`}
          style={{ paddingLeft: 8 + (e.kind === "heading" ? (e.level - 1) * 12 : 36) }}
          onClick={() => jump(e.pos)}
          title={e.text}
        >
          {e.kind === "heading" ? (
            e.number && <span className="outline-number">{e.appendix && e.level === 1 ? e.refText : e.number}</span>
          ) : (
            <span className="outline-kind">{e.refText}</span>
          )}
          <span className="outline-text">{e.text}</span>
        </button>
      ))}
    </nav>
  );
}
