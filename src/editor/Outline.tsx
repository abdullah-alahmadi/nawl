// Outline of the document's headings, figures and tables; click to jump there.
import { useEditorState, type Editor } from "@tiptap/react";
import { TextSelection } from "@tiptap/pm/state";

interface Entry {
  kind: "heading" | "figure" | "table";
  level: number;
  text: string;
  pos: number;
  number: string;
}

/** Walks the document and numbers entries the way the template will. */
function collect(editor: Editor, numberedDepth: number, perChapter: boolean): Entry[] {
  const entries: Entry[] = [];
  const counters = [0, 0, 0, 0];
  let figure = 0;
  let table = 0;
  editor.state.doc.forEach((node, pos) => {
    if (node.type.name === "heading") {
      const level = Math.min(4, Math.max(1, node.attrs.level as number));
      counters[level - 1]++;
      for (let i = level; i < 4; i++) counters[i] = 0;
      if (level === 1 && perChapter) figure = table = 0;
      const number = level <= numberedDepth ? counters.slice(0, level).join(".") : "";
      entries.push({ kind: "heading", level, text: node.textContent || "(untitled)", pos, number });
    } else if (node.type.name === "figure" || node.type.name === "tableFigure") {
      const isFigure = node.type.name === "figure";
      const n = isFigure ? ++figure : ++table;
      const number = perChapter ? `${counters[0]}.${n}` : String(n);
      entries.push({
        kind: isFigure ? "figure" : "table",
        level: 5,
        text: node.firstChild?.textContent || "(no caption)",
        pos,
        number,
      });
    }
  });
  return entries;
}

export function Outline({
  editor,
  numberedDepth,
  perChapter,
  figureLabel,
  tableLabel,
}: {
  editor: Editor;
  numberedDepth: number;
  perChapter: boolean;
  figureLabel: string;
  tableLabel: string;
}) {
  const entries = useEditorState({
    editor,
    selector: ({ editor }) => collect(editor, numberedDepth, perChapter),
    equalityFn: (a, b) => JSON.stringify(a) === JSON.stringify(b),
  });

  const jump = (pos: number) => {
    const { state, view } = editor;
    // Put the cursor at the start of the item's text and scroll it to the top.
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
          className={`outline-item outline-${e.kind}`}
          style={{ paddingLeft: 8 + (e.kind === "heading" ? (e.level - 1) * 12 : 36) }}
          onClick={() => jump(e.pos)}
          title={e.text}
        >
          {e.kind !== "heading" && <span className="outline-kind">{e.kind === "figure" ? figureLabel : tableLabel} </span>}
          {e.number && <span className="outline-number">{e.number}</span>}
          <span className="outline-text">{e.text}</span>
        </button>
      ))}
    </nav>
  );
}
