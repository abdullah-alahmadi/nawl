import { useCallback, useEffect, useRef, useState } from "react";
import { useEditor, useEditorState, EditorContent, type Editor, type JSONContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Equation } from "./editor/Equation";
import { PdfPreview } from "./preview/PdfPreview";
import * as api from "./api";
import type { AppError } from "./api";
import "./App.css";

const COMPILE_DEBOUNCE_MS = 700;
const DOC_FILTER = [{ name: "Texloom document", extensions: ["loom"] }];

const SAMPLE: JSONContent = {
  type: "doc",
  content: [
    { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Introduction" }] },
    {
      type: "paragraph",
      content: [
        { type: "text", text: "This report describes our senior design project. Text can be " },
        { type: "text", text: "bold", marks: [{ type: "bold" }] },
        { type: "text", text: ", " },
        { type: "text", text: "italic", marks: [{ type: "italic" }] },
        { type: "text", text: " or contain specials like 50% & $5 without breaking LaTeX." },
      ],
    },
    { type: "heading", attrs: { level: 2 }, content: [{ type: "text", text: "Model" }] },
    { type: "paragraph", content: [{ type: "text", text: "The energy of the system is given by:" }] },
    { type: "equation", attrs: { latex: "E=\\frac{1}{2}mv^2+mgh" } },
    {
      type: "bulletList",
      content: ["Click the equation to edit it visually", "Type $$ and a space on an empty line for a new one"].map(
        (text) => ({ type: "listItem", content: [{ type: "paragraph", content: [{ type: "text", text }] }] }),
      ),
    },
  ],
};

type Status = { state: "idle" | "compiling" | "ok" } | { state: "error"; error: AppError };

export default function App() {
  const [pdf, setPdf] = useState<ArrayBuffer | null>(null);
  const [status, setStatus] = useState<Status>({ state: "idle" });
  const [path, setPath] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [showLatex, setShowLatex] = useState(false);
  const [latex, setLatex] = useState("");
  const requestId = useRef(0);
  const timer = useRef<number | undefined>(undefined);

  const compile = useCallback(async (doc: JSONContent) => {
    const id = ++requestId.current;
    setStatus({ state: "compiling" });
    try {
      const bytes = await api.renderPdf(doc);
      if (id !== requestId.current) return; // a newer compile superseded this one
      setPdf(bytes);
      setStatus({ state: "ok" });
    } catch (e) {
      if (id === requestId.current) setStatus({ state: "error", error: api.asAppError(e) });
    }
  }, []);

  const scheduleCompile = useCallback(
    (editor: Editor) => {
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(() => compile(editor.getJSON()), COMPILE_DEBOUNCE_MS);
    },
    [compile],
  );

  const editor = useEditor({
    extensions: [StarterKit, Equation],
    content: SAMPLE,
    onCreate: ({ editor }) => compile(editor.getJSON()),
    onUpdate: ({ editor }) => {
      setDirty(true);
      scheduleCompile(editor);
    },
  });

  // Keep the LaTeX source view current while it's open.
  useEffect(() => {
    if (!showLatex || !editor) return;
    const refresh = () => api.toLatex(editor.getJSON()).then(setLatex);
    refresh();
    editor.on("update", refresh);
    return () => void editor.off("update", refresh);
  }, [showLatex, editor]);

  const report = (e: unknown) => setStatus({ state: "error", error: api.asAppError(e) });

  const saveAs = useCallback(async () => {
    if (!editor) return;
    const target = await save({ filters: DOC_FILTER, defaultPath: path ?? "report.loom" });
    if (!target) return;
    await api.saveDocument(target, editor.getJSON()).then(() => {
      setPath(target);
      setDirty(false);
    }, report);
  }, [editor, path]);

  const saveDoc = useCallback(async () => {
    if (!editor) return;
    if (!path) return saveAs();
    await api.saveDocument(path, editor.getJSON()).then(() => setDirty(false), report);
  }, [editor, path, saveAs]);

  const openDoc = async () => {
    if (!editor) return;
    const target = await open({ filters: DOC_FILTER, multiple: false, directory: false });
    if (!target) return;
    try {
      const doc = await api.openDocument(target);
      editor.commands.setContent(doc, { emitUpdate: false });
      setPath(target);
      setDirty(false);
      compile(doc);
    } catch (e) {
      report(e);
    }
  };

  const exportLatex = async () => {
    if (!editor) return;
    const target = await save({ filters: [{ name: "LaTeX", extensions: ["tex"] }], defaultPath: "report.tex" });
    if (target) await api.exportTex(target, editor.getJSON()).catch(report);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (e.shiftKey) saveAs();
        else saveDoc();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [saveDoc, saveAs]);

  const fileName = path ? path.split(/[\\/]/).pop() : "Untitled";

  return (
    <div className="app">
      <header className="toolbar">
        <div className="group">
          <button onClick={openDoc}>Open</button>
          <button onClick={saveDoc} title="Ctrl+S">Save</button>
          <button onClick={saveAs} title="Ctrl+Shift+S">Save As</button>
          <button onClick={exportLatex}>Export .tex</button>
        </div>
        {editor && <FormatBar editor={editor} />}
        <div className="group right">
          <button className={showLatex ? "active" : ""} onClick={() => setShowLatex((v) => !v)}>
            LaTeX source
          </button>
          <span className="filename">
            {fileName}
            {dirty ? " •" : ""}
          </span>
          <StatusBadge status={status} />
        </div>
      </header>

      <main className="panes">
        <section className="editor-pane">
          <div className="page">
            <EditorContent editor={editor} />
          </div>
        </section>
        <section className="preview-pane">
          {status.state === "error" && <ErrorPanel error={status.error} />}
          {showLatex ? <pre className="latex-source">{latex}</pre> : <PdfPreview pdf={pdf} />}
        </section>
      </main>
    </div>
  );
}

function FormatBar({ editor }: { editor: Editor }) {
  // Re-render only when the formatting state at the cursor changes.
  const s = useEditorState({
    editor,
    selector: ({ editor }) => ({
      bold: editor.isActive("bold"),
      italic: editor.isActive("italic"),
      code: editor.isActive("code"),
      h1: editor.isActive("heading", { level: 1 }),
      h2: editor.isActive("heading", { level: 2 }),
      h3: editor.isActive("heading", { level: 3 }),
      bullet: editor.isActive("bulletList"),
      ordered: editor.isActive("orderedList"),
      quote: editor.isActive("blockquote"),
      canUndo: editor.can().undo(),
      canRedo: editor.can().redo(),
    }),
  });
  const c = () => editor.chain().focus();
  const B = ({ on, label, title, run }: { on?: boolean; label: string; title: string; run: () => void }) => (
    <button className={on ? "active" : ""} title={title} onMouseDown={(e) => e.preventDefault()} onClick={run}>
      {label}
    </button>
  );
  return (
    <div className="group">
      <B label="↶" title="Undo (Ctrl+Z)" run={() => c().undo().run()} on={false} />
      <B label="↷" title="Redo (Ctrl+Shift+Z)" run={() => c().redo().run()} on={false} />
      <span className="sep" />
      <B label="H1" title="Section" on={s.h1} run={() => c().toggleHeading({ level: 1 }).run()} />
      <B label="H2" title="Subsection" on={s.h2} run={() => c().toggleHeading({ level: 2 }).run()} />
      <B label="H3" title="Subsubsection" on={s.h3} run={() => c().toggleHeading({ level: 3 }).run()} />
      <span className="sep" />
      <B label="B" title="Bold (Ctrl+B)" on={s.bold} run={() => c().toggleBold().run()} />
      <B label="I" title="Italic (Ctrl+I)" on={s.italic} run={() => c().toggleItalic().run()} />
      <B label="<>" title="Code (Ctrl+E)" on={s.code} run={() => c().toggleCode().run()} />
      <span className="sep" />
      <B label="• List" title="Bullet list" on={s.bullet} run={() => c().toggleBulletList().run()} />
      <B label="1. List" title="Numbered list" on={s.ordered} run={() => c().toggleOrderedList().run()} />
      <B label="❝" title="Quote" on={s.quote} run={() => c().toggleBlockquote().run()} />
      <B label="∑ Equation" title="Insert equation ($$ + space)" run={() => c().insertEquation().run()} />
    </div>
  );
}

function StatusBadge({ status }: { status: Status }) {
  const text = {
    idle: "Ready",
    compiling: "Compiling…",
    ok: "Up to date",
    error: status.state === "error" && status.error.kind === "texErrors" ? `${status.error.errors.length} error(s)` : "Error",
  }[status.state];
  return <span className={`status status-${status.state}`}>{text}</span>;
}

function ErrorPanel({ error }: { error: AppError }) {
  if (error.kind === "texErrors") {
    return (
      <div className="error-panel">
        <strong>LaTeX reported errors (the preview shows the last good version):</strong>
        <ul>
          {error.errors.map((e, i) => (
            <li key={i}>
              {e.line != null && <span className="muted">line {e.line}: </span>}
              {e.message}
            </li>
          ))}
        </ul>
        <details>
          <summary>Full log</summary>
          <pre>{error.log}</pre>
        </details>
      </div>
    );
  }
  return (
    <div className="error-panel">
      <strong>{error.kind === "tectonicMissing" ? "Tectonic not found" : "Error"}:</strong> {error.message}
    </div>
  );
}
