import { useCallback, useEffect, useRef, useState } from "react";
import { useEditor, useEditorState, EditorContent, type Editor, type JSONContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { TableKit } from "@tiptap/extension-table";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Equation } from "./editor/Equation";
import { MetadataForm } from "./editor/MetadataForm";
import { Caption, CaptionedTable, Figure, pickFigureImage } from "./editor/Figures";
import { Outline } from "./editor/Outline";
import { PAGE_WIDTH_PX, templateCss } from "./editor/templateStyle";
import { TemplateDesigner } from "./designer/TemplateDesigner";
import { PdfPreview } from "./preview/PdfPreview";
import * as api from "./api";
import type { AppError } from "./api";
import "./App.css";

const COMPILE_DEBOUNCE_MS = 700;
/** Designer edits are validated after this pause, then the preview recompiles. */
const TEMPLATE_DEBOUNCE_MS = 400;
const DOC_FILTER = [{ name: "Texloom document", extensions: ["loom"] }];
const TEMPLATE_FILTER = [{ name: "Texloom template", extensions: ["json"] }];

const EMPTY_DOC: JSONContent = { type: "doc", content: [{ type: "paragraph" }] };

/** Loads a document into the editor; throws a readable error instead of silently
 *  showing an empty page when the document doesn't fit the editor's schema. */
function loadDocument(editor: Editor, doc: JSONContent) {
  try {
    editor.schema.nodeFromJSON(doc).check();
  } catch (e) {
    throw new Error(`This document can't be opened: ${e instanceof Error ? e.message : String(e)}`);
  }
  editor.commands.setContent(doc, { emitUpdate: false });
}

type Status = { state: "idle" | "compiling" | "ok" } | { state: "error"; error: AppError };

export default function App() {
  const [pdf, setPdf] = useState<ArrayBuffer | null>(null);
  const [status, setStatus] = useState<Status>({ state: "idle" });
  const [path, setPath] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [showLatex, setShowLatex] = useState(false);
  const [showInfo, setShowInfo] = useState(false);
  const [showOutline, setShowOutline] = useState(true);
  const [latex, setLatex] = useState("");
  const [templateInfo, setTemplateInfo] = useState<api.TemplateInfo | null>(null);
  // Set when the template file on disk has errors; the last valid version stays in use.
  const [templateProblem, setTemplateProblem] = useState<{ path: string; issues: api.Issue[] } | null>(null);
  const [showTemplateMenu, setShowTemplateMenu] = useState(false);
  const [mode, setMode] = useState<"document" | "template">("document");
  // The designer edits a draft; valid drafts become the active template right away.
  const [draft, setDraft] = useState<api.Template | null>(null);
  const [draftDirty, setDraftDirty] = useState(false);
  const [draftIssues, setDraftIssues] = useState<api.Issue[]>([]);
  // "New copy" drafts must be saved to a new file, never over the original.
  const [draftNeedsSaveAs, setDraftNeedsSaveAs] = useState(false);
  const draftTimer = useRef<number | undefined>(undefined);
  const template = templateInfo?.template ?? null;
  const [metadata, setMetadataState] = useState<api.Metadata>({});
  const [issues, setIssues] = useState<api.Issue[]>([]);
  // Refs give debounced callbacks the latest values without re-subscribing.
  const metadataRef = useRef<api.Metadata>({});
  const editorRef = useRef<Editor | null>(null);
  const requestId = useRef(0);
  const timer = useRef<number | undefined>(undefined);
  const started = useRef(false);
  const editorScroll = useRef<HTMLDivElement>(null);
  const [pageZoom, setPageZoom] = useState(1);

  const compile = useCallback(async () => {
    const editor = editorRef.current;
    if (!editor) return;
    const id = ++requestId.current;
    setStatus({ state: "compiling" });
    try {
      const bytes = await api.renderPdf(editor.getJSON(), metadataRef.current);
      if (id !== requestId.current) return; // a newer compile superseded this one
      setPdf(bytes);
      setStatus({ state: "ok" });
    } catch (e) {
      if (id === requestId.current) setStatus({ state: "error", error: api.asAppError(e) });
    }
  }, []);

  const scheduleCompile = useCallback(() => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(compile, COMPILE_DEBOUNCE_MS);
  }, [compile]);

  const setMetadata = useCallback((m: api.Metadata) => {
    metadataRef.current = m;
    setMetadataState(m);
    api.checkMetadata(m).then(setIssues, () => setIssues([]));
  }, []);

  const editor = useEditor({
    extensions: [
      StarterKit,
      Equation,
      Caption,
      Figure,
      CaptionedTable,
      TableKit.configure({ table: { resizable: false } }),
    ],
    content: EMPTY_DOC,
    onUpdate: () => {
      setDirty(true);
      scheduleCompile();
    },
  });
  editorRef.current = editor;

  // Zoom the fixed-width page to fit the editor pane.
  useEffect(() => {
    const el = editorScroll.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setPageZoom(Math.min(1.25, Math.max(0.4, (el.clientWidth - 48) / PAGE_WIDTH_PX))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Startup: load the template and open the sample report.
  useEffect(() => {
    if (!editor || started.current) return;
    started.current = true; // React StrictMode runs effects twice in development
    Promise.all([api.getTemplate(), api.sampleDocument()])
      .then(([t, sample]) => {
        setTemplateInfo(t);
        setDraft(t.template);
        setMetadata(sample.metadata);
        loadDocument(editor, sample.doc);
        compile();
      })
      .catch((e) => setStatus({ state: "error", error: api.asAppError(e) }));
  }, [editor, compile, setMetadata]);

  // A new active template: refresh everything that depends on it.
  const applyTemplate = useCallback(
    (info: api.TemplateInfo) => {
      setTemplateInfo(info);
      setTemplateProblem(null);
      setDraft(info.template);
      setDraftDirty(false);
      setDraftNeedsSaveAs(false);
      setDraftIssues(info.warnings);
      setMetadata(metadataRef.current); // re-check required fields against the new template
      compile();
    },
    [compile, setMetadata],
  );

  // Live reload: the template file was edited on disk.
  useEffect(() => {
    const unlisten = api.onTemplateReloaded((e) => {
      if (e.kind === "reloaded") applyTemplate(e.info);
      else setTemplateProblem({ path: e.path, issues: e.issues });
    });
    return () => void unlisten.then((f) => f());
  }, [applyTemplate]);

  const openTemplateFile = async () => {
    setShowTemplateMenu(false);
    const target = await open({ filters: TEMPLATE_FILTER, multiple: false, directory: false });
    if (!target) return;
    try {
      applyTemplate(await api.openTemplate(target));
    } catch (e) {
      const err = api.asAppError(e);
      if (err.kind === "templateInvalid") setTemplateProblem({ path: err.path, issues: err.issues });
      else report(e);
    }
  };

  const switchToBuiltin = async () => {
    setShowTemplateMenu(false);
    applyTemplate(await api.switchToBuiltinTemplate());
  };

  // Designer edits: validate after a pause; valid drafts go live in the preview.
  const onDraftChange = useCallback(
    (t: api.Template) => {
      setDraft(t);
      setDraftDirty(true);
      window.clearTimeout(draftTimer.current);
      draftTimer.current = window.setTimeout(async () => {
        try {
          const warnings = await api.setTemplate(t);
          setDraftIssues(warnings);
          setTemplateInfo((prev) => (prev ? { ...prev, template: t, warnings } : prev));
          setMetadata(metadataRef.current);
          compile();
        } catch (e) {
          const err = api.asAppError(e);
          if (err.kind === "templateInvalid") setDraftIssues(err.issues);
          else setStatus({ state: "error", error: err });
        }
      }, TEMPLATE_DEBOUNCE_MS);
    },
    [compile, setMetadata],
  );

  const saveTemplateAs = useCallback(async () => {
    if (!draft) return;
    const target = await save({ filters: TEMPLATE_FILTER, defaultPath: `${draft.meta.id}.json` });
    if (!target) return;
    try {
      applyTemplate(await api.saveTemplate(target, draft));
    } catch (e) {
      const err = api.asAppError(e);
      if (err.kind === "templateInvalid") setDraftIssues(err.issues);
      else setStatus({ state: "error", error: err });
    }
  }, [draft, applyTemplate]);

  const saveTemplate = useCallback(async () => {
    if (!draft) return;
    const target = templateInfo?.path;
    if (!target || draftNeedsSaveAs) return saveTemplateAs();
    try {
      applyTemplate(await api.saveTemplate(target, draft));
    } catch (e) {
      const err = api.asAppError(e);
      if (err.kind === "templateInvalid") setDraftIssues(err.issues);
      else setStatus({ state: "error", error: err });
    }
  }, [draft, templateInfo, draftNeedsSaveAs, saveTemplateAs, applyTemplate]);

  const newTemplateCopy = () => {
    if (!draft) return;
    const copy = structuredClone(draft);
    copy.meta.id = `${copy.meta.id}-copy`;
    copy.meta.name = `${copy.meta.name} (copy)`;
    copy.meta.version = "0.1.0";
    setDraftNeedsSaveAs(true);
    onDraftChange(copy);
  };

  const onMetadataChange = (m: api.Metadata) => {
    setMetadata(m);
    setDirty(true);
    scheduleCompile();
  };

  // Keep the LaTeX source view current while it's open.
  useEffect(() => {
    if (!showLatex || !editor) return;
    const refresh = () => api.toLatex(editor.getJSON(), metadataRef.current).then(setLatex);
    refresh();
    editor.on("update", refresh);
    return () => void editor.off("update", refresh);
  }, [showLatex, editor, metadata]);

  const report = (e: unknown) => setStatus({ state: "error", error: api.asAppError(e) });

  const saveAs = useCallback(async () => {
    if (!editor) return;
    const target = await save({ filters: DOC_FILTER, defaultPath: path ?? "report.loom" });
    if (!target) return;
    await api.saveDocument(target, editor.getJSON(), metadataRef.current).then(() => {
      setPath(target);
      setDirty(false);
    }, report);
  }, [editor, path]);

  const saveDoc = useCallback(async () => {
    if (!editor) return;
    if (!path) return saveAs();
    await api.saveDocument(path, editor.getJSON(), metadataRef.current).then(() => setDirty(false), report);
  }, [editor, path, saveAs]);

  const openDoc = async () => {
    if (!editor) return;
    const target = await open({ filters: DOC_FILTER, multiple: false, directory: false });
    if (!target) return;
    try {
      const opened = await api.openDocument(target);
      loadDocument(editor, opened.doc);
      metadataRef.current = opened.metadata;
      setPath(target);
      setDirty(false);
      if (opened.template) applyTemplate(opened.template);
      else {
        setMetadata(opened.metadata);
        compile();
      }
    } catch (e) {
      report(e);
    }
  };

  const exportPdfFile = async () => {
    if (!editor) return;
    const name = (path ? path.split(/[\\/]/).pop()!.replace(/\.loom$/, "") : "report") + ".pdf";
    const target = await save({ filters: [{ name: "PDF", extensions: ["pdf"] }], defaultPath: name });
    if (target) await api.exportPdf(target, editor.getJSON(), metadataRef.current).catch(report);
  };

  const exportLatex = async () => {
    if (!editor) return;
    const target = await save({ filters: [{ name: "LaTeX", extensions: ["tex"] }], defaultPath: "report.tex" });
    if (target) await api.exportTex(target, editor.getJSON(), metadataRef.current).catch(report);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        if (mode === "template") {
          if (e.shiftKey) saveTemplateAs();
          else saveTemplate();
        } else if (e.shiftKey) saveAs();
        else saveDoc();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mode, saveDoc, saveAs, saveTemplate, saveTemplateAs]);

  const fileName = path ? path.split(/[\\/]/).pop() : "Untitled";
  const missing = issues.filter((i) => i.path.startsWith("metadata.") && i.message.includes("required")).length;

  return (
    <div className="app">
      <header className="toolbar">
        <div className="mode-switch" role="tablist" aria-label="Mode">
          <button role="tab" aria-selected={mode === "document"} className={mode === "document" ? "active" : ""} onClick={() => setMode("document")}>
            Document
          </button>
          <button role="tab" aria-selected={mode === "template"} className={mode === "template" ? "active" : ""} onClick={() => setMode("template")}>
            Template{draftDirty && " •"}
            {draftIssues.some((i) => i.severity === "error") && <span className="badge">!</span>}
          </button>
        </div>
        <div className="group">
          <button onClick={openDoc}>Open</button>
          <button onClick={saveDoc} title="Ctrl+S">Save</button>
          <button onClick={saveAs} title="Ctrl+Shift+S">Save As</button>
          <button onClick={exportPdfFile}>Export PDF</button>
          <button onClick={exportLatex} title="Writes the .tex file, the template's .cls and images next to each other">
            Export .tex
          </button>
        </div>
        {editor && mode === "document" && <FormatBar editor={editor} onError={report} />}
        <div className="group right">
          <div className="menu-anchor">
            <button
              className={showTemplateMenu ? "active" : ""}
              onClick={() => setShowTemplateMenu((v) => !v)}
              title={templateInfo?.path ?? "Built-in template"}
            >
              Template: {template?.meta.name ?? "…"}
              {templateProblem && <span className="badge">!</span>} ▾
            </button>
            {showTemplateMenu && (
              <div className="menu" role="menu">
                <div className="menu-info">
                  <strong>{template?.meta.name}</strong>
                  <span className="muted">{templateInfo?.path ?? "Built into Texloom"}</span>
                  {templateInfo?.path && <span className="muted">Reloads automatically when the file changes.</span>}
                </div>
                <button role="menuitem" onClick={openTemplateFile}>
                  Open template file…
                </button>
                <button role="menuitem" onClick={switchToBuiltin}>
                  Use built-in “Senior Design Report”
                </button>
              </div>
            )}
          </div>
          <button hidden={mode !== "document"} className={showOutline ? "active" : ""} onClick={() => setShowOutline((v) => !v)}>
            Outline
          </button>
          <button
            hidden={mode !== "document"}
            className={showInfo ? "active" : ""}
            onClick={() => setShowInfo((v) => !v)}
            title="Title, team members, abstract and other template fields"
          >
            Document info{missing > 0 && <span className="badge">{missing}</span>}
          </button>
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

      <main className={`panes${mode === "template" ? " template-mode" : ""}`}>
        {mode === "template" && templateInfo && draft && (
          <section className="editor-pane designer-pane">
            <TemplateDesigner
              info={templateInfo}
              draft={draft}
              dirty={draftDirty}
              issues={draftIssues}
              metadata={metadata}
              needsSaveAs={draftNeedsSaveAs}
              onChange={onDraftChange}
              onSave={saveTemplate}
              onSaveAs={saveTemplateAs}
              onNewCopy={newTemplateCopy}
            />
          </section>
        )}
        {/* Kept mounted (just hidden) in template mode so the editor keeps its state. */}
        <section className={`editor-pane${showOutline ? " with-outline" : ""}`} hidden={mode !== "document"}>
          {template && <style>{templateCss(template, ".page")}</style>}
          {showOutline && editor && template && (
            <Outline
              editor={editor}
              numberedDepth={template.structure.numberedDepth}
              perChapter={template.structure.topLevel === "chapter" && template.structure.perChapterNumbering}
              figureLabel={template.captions.figureLabel}
              tableLabel={template.captions.tableLabel}
            />
          )}
          <div className="editor-scroll" ref={editorScroll} style={{ ["--page-zoom" as string]: pageZoom }}>
            {showInfo && template && (
              <MetadataForm template={template} metadata={metadata} onChange={onMetadataChange} />
            )}
            <div className="page">
              <EditorContent editor={editor} />
            </div>
          </div>
        </section>
        <section className="preview-pane">
          {templateProblem && <TemplateProblemPanel problem={templateProblem} onDismiss={() => setTemplateProblem(null)} />}
          {status.state === "error" && <ErrorPanel error={status.error} />}
          {showLatex ? <pre className="latex-source">{latex}</pre> : <PdfPreview pdf={pdf} />}
        </section>
      </main>
    </div>
  );
}

function FormatBar({ editor, onError }: { editor: Editor; onError: (e: unknown) => void }) {
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
      inTable: editor.isActive("table"),
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
      <B
        label="▣ Figure"
        title="Insert an image with a caption"
        run={() =>
          pickFigureImage().then((src) => {
            if (src) c().insertFigure(src).run();
          }, onError)
        }
      />
      <B label="▦ Table" title="Insert a table with a caption" run={() => c().insertCaptionedTable(3, 3).run()} />
      {s.inTable && (
        <span className="table-tools" role="group" aria-label="Table">
          <span className="sep" />
          <B label="+ Row" title="Add a row below" run={() => c().addRowAfter().run()} />
          <B label="+ Col" title="Add a column to the right" run={() => c().addColumnAfter().run()} />
          <B label="− Row" title="Delete this row" run={() => c().deleteRow().run()} />
          <B label="− Col" title="Delete this column" run={() => c().deleteColumn().run()} />
          <B label="Header" title="Toggle the header row" run={() => c().toggleHeaderRow().run()} />
          <B label="Delete table" title="Delete the table and its caption" run={() => c().deleteCaptionedTable().run()} />
        </span>
      )}
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

function TemplateProblemPanel({
  problem,
  onDismiss,
}: {
  problem: { path: string; issues: api.Issue[] };
  onDismiss: () => void;
}) {
  return (
    <div className="error-panel">
      <button className="dismiss" onClick={onDismiss} title="Dismiss">
        ×
      </button>
      <strong>The template has errors, so the last valid version is still in use:</strong>
      <div className="muted path">{problem.path}</div>
      <ul>
        {problem.issues
          .filter((i) => i.severity === "error")
          .map((i, n) => (
            <li key={n}>
              {i.path && <code>{i.path}</code>} {i.message}
            </li>
          ))}
      </ul>
    </div>
  );
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
  if (error.kind === "templateInvalid") {
    return <TemplateProblemPanel problem={error} onDismiss={() => {}} />;
  }
  return (
    <div className="error-panel">
      <strong>{error.kind === "tectonicMissing" ? "Tectonic not found" : "Error"}:</strong> {error.message}
    </div>
  );
}
