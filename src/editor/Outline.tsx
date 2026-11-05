// Outline of the document's headings, figures and tables; click to jump there.
import type { Editor } from "@tiptap/react";
import { TextSelection } from "@tiptap/pm/state";
import { useNumbering } from "./Academic";

export function Outline({ editor }: { editor: Editor }) {
  const entries = useNumbering(editor).filter((e) => e.kind !== "equation");

  const jump = (pos: number) => {
    const { state, view } = editor;
    const target = Math.min(pos + 1, state.doc.content.size);
    view.dispatch(state.tr.setSelection(TextSelection.near(state.doc.resolve(target))));
    view.focus();
    const dom = view.nodeDOM(pos);
    if (dom instanceof HTMLElement) dom.scrollIntoView({ block: "start", behavior: "smooth" });
  };

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
