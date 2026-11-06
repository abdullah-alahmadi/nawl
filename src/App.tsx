import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useEditor, useEditorState, EditorContent, type Editor, type JSONContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { EditorState, Selection } from "@tiptap/pm/state";
import { TableKit } from "@tiptap/extension-table";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Equation } from "./editor/Equation";
import { InlineMath } from "./editor/InlineMath";
import { RawLatex } from "./editor/RawLatex";
import { EmptyLines } from "./editor/EmptyLines";
import { MetadataForm } from "./editor/MetadataForm";
import { Caption, CaptionedTable, Figure, pickFigureImage } from "./editor/Figures";
import { Outline } from "./editor/Outline";
import { AppendixStart, Citation, CrossRef, TexloomIds, setTexloomContext } from "./editor/Academic";
import { CitePicker, ReferencesPanel, RefPicker } from "./editor/Pickers";
import { PAGE_WIDTH_PX, templateCss, type DeckInfo } from "./editor/templateStyle";
import { useDocumentImage } from "./editor/Figures";
import { toAuthors } from "./editor/AuthorsEditor";
import { DeckDocument, SLIDE_EXTENSIONS } from "./editor/Slides";
import { Spaces } from "./editor/Spaces";
import { Search } from "./editor/Search";
import { Flash, flashBlock } from "./editor/Flash";
import { SearchBar } from "./editor/SearchBar";
import { TemplateDesigner } from "./designer/TemplateDesigner";
import { SaveTemplateDialog, type SaveAsChoice } from "./designer/SaveTemplateDialog";
import { withKitFiles } from "./designer/ContentSections";
import { PdfPreview } from "./preview/PdfPreview";
import { PagePreview } from "./preview/PagePreview";
import * as api from "./api";
import type { AppError } from "./api";
import { useSettings } from "./settings";
import { SettingsDialog } from "./SettingsDialog";
import { NewDocumentDialog } from "./NewDocumentDialog";
import { ThemePicker } from "./ThemePicker";
import type { PresentationSettings } from "./bindings/PresentationSettings";
import { Menu, MenuHeading, MenuItem, MenuSeparator } from "./ui/Menu";
import {
  BookMarked, Bold, FileInput, FileOutput, Sparkles, Braces, ChevronDown, CircleAlert, CircleCheck, Code, Columns3, FileCode2, FileDown, FilePlus2,
  FileText, FolderOpen, Hash, History, IdCard, Image, Italic, LayoutTemplate, Library, Link2, List, ListOrdered, ListTree,
  LoaderCircle, Menu as MenuIcon, PenLine, Play, Quote, Redo2, Rows3, Save, Settings as SettingsIcon, Palette, Columns2, SquareStack, Pause, GalleryVerticalEnd, Sigma, Superscript,
  Table, TextQuote, Trash2, TriangleAlert, Undo2, X, ZoomIn, ZoomOut, LocateFixed, TableCellsMerge, TableCellsSplit,
} from "lucide-react";
import "./App.css";

/** Designer edits are validated after this pause, then the preview recompiles. */
const TEMPLATE_DEBOUNCE_MS = 400;
/** Unsaved changes are copied for crash recovery after this pause in typing. */
const RECOVERY_DELAY_MS = 3000;
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
  // A fresh undo history: Ctrl+Z must not step back into the previous document.
  const { state } = editor;
  editor.view.updateState(EditorState.create({ doc: state.doc, plugins: state.plugins, selection: Selection.atStart(state.doc) }));
}

const fileNameOf = (p: string) => p.split(/[\\/]/).pop() ?? p;

/** Puts the cursor on the document's first line, once dialogs have closed. */
function focusStart(editor: Editor, scroller: HTMLElement | null) {
  window.setTimeout(() => {
    if (editor.isDestroyed) return;
    const { state, view } = editor;
    view.dispatch(state.tr.setSelection(Selection.atStart(state.doc)));
    view.focus();
    scroller?.scrollTo({ top: 0 });
  }, 0);
}

/** "today at 14:05" / "on 3 Nov at 14:05" for the restore prompt. */
function whenText(secs: number) {
  const d = new Date(secs * 1000);
  const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return d.toDateString() === new Date().toDateString()
    ? `today at ${time}`
    : `on ${d.toLocaleDateString([], { day: "numeric", month: "short" })} at ${time}`;
}

/** `name` inside `folder` (or just `name` without a folder), for dialog defaults. */
const inFolder = (folder: string, name: string) => (folder ? `${folder.replace(/[\\/]+$/, "")}/${name}` : name);

const countWords = (text: string) => (text.match(/[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*/gu) ?? []).length;

type SidePanel = "outline" | "info" | "refs" | "checks";

type Status = { state: "idle" | "compiling" | "ok" } | { state: "error"; error: AppError };

export default function App() {
  const [pdf, setPdf] = useState<ArrayBuffer | null>(null);
  const [status, setStatus] = useState<Status>({ state: "idle" });
  const [path, setPath] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [showLatex, setShowLatex] = useState(false);
  /** The find bar: closed, or open (with the replace row when asked for with Ctrl+H). */
  const [search, setSearchBar] = useState<{ replace: boolean } | null>(null);
  /** Design mode's preview: the compiled PDF, or the instant page preview. */
  const [designPreview, setDesignPreview] = useState<"pdf" | "page">("pdf");
  /** A place the PDF preview scrolls to and marks ("Show in PDF"). */
  const [pdfSpot, setPdfSpot] = useState<api.PdfSpot | null>(null);
  // One side panel at a time, chosen on the left rail.
  const [panel, setPanel] = useState<SidePanel | null>("outline");
  const togglePanel = (p: SidePanel) => setPanel((cur) => (cur === p ? null : p));
  const [showSettings, setShowSettings] = useState(false);
  const [showGallery, setShowGallery] = useState(false);
  // The gallery as the start window: also offers Open, recent files and the example.
  const [startWindow, setStartWindow] = useState(false);
  const { settings, update: updateSettings, addRecent, removeRecent, latest: settingsRef } = useSettings();
  const [compileMs, setCompileMs] = useState<number | null>(null);
  const [pageCount, setPageCount] = useState(0);
  const [previewZoom, setPreviewZoom] = useState(100);
  const [latex, setLatex] = useState("");
  const [templateInfo, setTemplateInfo] = useState<api.TemplateInfo | null>(null);
  // Set when the template file on disk has errors; the last valid version stays in use.
  const [templateProblem, setTemplateProblem] = useState<{ path: string; issues: api.Issue[] } | null>(null);
  const [builtins, setBuiltins] = useState<api.BuiltinTemplate[]>([]);
  const [installedTemplates, setInstalledTemplates] = useState<api.InstalledTemplate[]>([]);
  /** Installed classes of kinds Texloom can't write yet (letters, CVs…). */
  const [installedWaiting, setInstalledWaiting] = useState<api.InstalledList["waiting"]>([]);
  /** Beamer themes the installed TeX adds to the theme picker. */
  const [installedThemeCount, setInstalledThemeCount] = useState(0);
  /** The template window with only the installed-TeX templates (from the template menu). */
  const [showTexPicker, setShowTexPicker] = useState(false);
  /** The presentation theme picker. */
  const [showThemePicker, setShowThemePicker] = useState(false);
  /** Asked once per session whether to leave Tectonic for an installed-only theme. */
  const askedThemeEngine = useRef(false);
  const [userTemplates, setUserTemplates] = useState<api.UserTemplate[]>([]);
  const refreshUserTemplates = useCallback(() => api.listUserTemplates().then(setUserTemplates, () => setUserTemplates([])), []);
  /** The open "Save template as" dialog, answered through `resolve`. */
  const [saveAsRequest, setSaveAsRequest] = useState<{ defaultName: string; title?: string; resolve: (c: SaveAsChoice | null) => void } | null>(null);
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
  // The document's BibTeX and what it parses to.
  const [bibliography, setBibliographyState] = useState("");
  const bibRef = useRef("");
  const [bibInfo, setBibInfo] = useState<api.BibInfo>({ entries: [], error: null });
  const [picker, setPicker] = useState<null | "ref" | "cite">(null);
  const editorRef = useRef<Editor | null>(null);
  const requestId = useRef(0);
  const timer = useRef<number | undefined>(undefined);
  const started = useRef(false);
  const editorScroll = useRef<HTMLDivElement>(null);
  // Crash recovery: refs so timers and the close handler see current values.
  const dirtyRef = useRef(false);
  const draftDirtyRef = useRef(false);
  const pathRef = useRef<string | null>(null);
  const recoveryTimer = useRef<number | undefined>(undefined);
  // Recovery writes and discards run one after another, never out of order.
  const recoveryQueue = useRef<Promise<unknown>>(Promise.resolve());
  const [fitZoom, setFitZoom] = useState(1);
  const pageZoom = settings.pageZoom > 0 ? settings.pageZoom / 100 : fitZoom;

  const compile = useCallback(async () => {
    const editor = editorRef.current;
    if (!editor) return;
    const id = ++requestId.current;
    setStatus({ state: "compiling" });
    const t0 = performance.now();
    const state: api.DocState = { doc: editor.getJSON(), metadata: metadataRef.current, bibliography: bibRef.current };
    api.checkDocument(state).then(setIssues, () => setIssues([]));
    try {
      const bytes = await api.renderPdf(state);
      if (id !== requestId.current) return; // a newer compile superseded this one
      setPdf(bytes);
      setStatus({ state: "ok" });
      setCompileMs(performance.now() - t0);
    } catch (e) {
      if (id === requestId.current) setStatus({ state: "error", error: api.asAppError(e) });
    }
  }, []);

  const scheduleCompile = useCallback(() => {
    window.clearTimeout(timer.current);
    // With automatic compiling off, the preview waits for Compile (Ctrl+Enter).
    if (!settingsRef.current.autoCompile) return setStatus((s) => (s.state === "error" ? s : { state: "idle" }));
    timer.current = window.setTimeout(compile, settingsRef.current.compileDelayMs);
  }, [compile, settingsRef]);

  const setMetadata = useCallback((m: api.Metadata) => {
    metadataRef.current = m;
    setMetadataState(m);
  }, []);

  /** Everything the backend needs about the open document. */
  const docState = (): api.DocState => ({
    doc: editorRef.current?.getJSON() ?? EMPTY_DOC,
    metadata: metadataRef.current,
    bibliography: bibRef.current,
  });

  const queueRecovery = (op: () => Promise<unknown>) => {
    recoveryQueue.current = recoveryQueue.current.then(op).catch((e) => console.warn("crash recovery:", e));
    return recoveryQueue.current;
  };

  /** The document has unsaved changes: keep a copy for crash recovery after a pause. */
  const markDirty = useCallback(() => {
    setDirty(true);
    dirtyRef.current = true;
    window.clearTimeout(recoveryTimer.current);
    recoveryTimer.current = window.setTimeout(() => {
      if (dirtyRef.current && settingsRef.current.crashRecovery) queueRecovery(() => api.writeRecovery(pathRef.current, docState()));
    }, RECOVERY_DELAY_MS);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** Saved, or deliberately thrown away: the recovery copy is no longer needed. */
  const markClean = useCallback(() => {
    setDirty(false);
    dirtyRef.current = false;
    window.clearTimeout(recoveryTimer.current);
    return queueRecovery(() => api.discardRecovery());
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    pathRef.current = path;
  }, [path]);
  useEffect(() => {
    draftDirtyRef.current = draftDirty;
  }, [draftDirty]);

  // Bibliography changes: parse for the citation picker and on-screen labels.
  const setBibliography = useCallback((source: string) => {
    bibRef.current = source;
    setBibliographyState(source);
    api.parseBibliography(source).then((info) => {
      if (bibRef.current !== source) return; // a newer edit superseded this one
      setBibInfo(info);
      const editor = editorRef.current;
      if (editor && !info.error) setTexloomContext(editor, { bibEntries: info.entries });
    });
  }, []);

  const editor = useEditor({
    extensions: [
      StarterKit.configure({ document: false }),
      DeckDocument,
      ...SLIDE_EXTENSIONS,
      TexloomIds,
      CrossRef,
      Citation,
      AppendixStart,
      Equation,
      InlineMath,
      RawLatex,
      EmptyLines,
      Search,
      Flash,
      Spaces,
      Caption,
      Figure,
      CaptionedTable,
      TableKit.configure({ table: { resizable: false } }),
    ],
    content: EMPTY_DOC,
    onUpdate: () => {
      markDirty();
      scheduleCompile();
    },
  });
  editorRef.current = editor;

  // Zoom the fixed-width page to fit the editor pane.
  useEffect(() => {
    const el = editorScroll.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setFitZoom(Math.min(1.25, Math.max(0.4, (el.clientWidth - 48) / PAGE_WIDTH_PX))));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  // Startup: offer to restore unsaved changes from a crash, else show the start
  // window (templates, open, recent files) over an empty document.
  useEffect(() => {
    if (!editor || started.current) return;
    started.current = true; // React StrictMode runs effects twice in development
    (async () => {
      const t = await api.getTemplate();
      setTemplateInfo(t);
      setDraft(t.template);
      if (await offerRecovery(editor)) return;
      const blank = await api.newDocument();
      loadDocument(editor, blank.doc);
      setPanel("info"); // like any new document: start with its title, authors…
      compile();
      setStartWindow(true);
      setShowGallery(true);
    })().catch((e) => setStatus({ state: "error", error: api.asAppError(e) }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor, compile, setMetadata, setBibliography]);

  /** Restores the changes a crash left behind, if the user wants them. */
  const offerRecovery = async (editor: Editor): Promise<boolean> => {
    const info = await api.recoveryInfo().catch(() => null);
    if (!info) return false;
    const name = info.originalPath ? `“${fileNameOf(info.originalPath)}”` : "an unsaved document";
    const when = info.savedAt ? ` (last change ${whenText(info.savedAt)})` : "";
    const restore = await ask(`Texloom closed before you saved your changes to ${name}${when}. Restore them?`, {
      title: "Restore unsaved changes",
      kind: "warning",
      okLabel: "Restore",
      cancelLabel: "Discard",
    });
    if (!restore) {
      await markClean();
      return false;
    }
    const r = await api.restoreRecovery();
    loadDocument(editor, r.doc);
    focusStart(editor, editorScroll.current);
    setBibliography(r.bibliography ?? "");
    metadataRef.current = r.metadata;
    setPath(r.originalPath);
    if (r.originalPath) addRecent(r.originalPath);
    // Still unsaved: the recovery copy stays until the user saves.
    setDirty(true);
    dirtyRef.current = true;
    if (r.template) applyTemplate(r.template);
    else {
      setMetadata(r.metadata);
      compile();
    }
    return true;
  };

  // Closing with unsaved changes asks first.
  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async (e) => {
      const doc = dirtyRef.current;
      const tpl = draftDirtyRef.current;
      if (!doc && !tpl) return;
      const what = doc && tpl ? "The document and the template have" : doc ? "The document has" : "The template has";
      const close = await ask(`${what} unsaved changes. Close without saving?`, {
        title: "Unsaved changes",
        kind: "warning",
        okLabel: "Close without saving",
        cancelLabel: "Cancel",
      });
      if (!close) e.preventDefault();
      else if (doc) await markClean();
    });
    return () => void unlisten.then((f) => f());
  }, [markClean]);

  useEffect(() => {
    api.listBuiltinTemplates().then(setBuiltins, () => setBuiltins([]));
    api.listInstalledTemplates().then(
      (l) => {
        setInstalledTemplates(l.templates);
        setInstalledWaiting(l.waiting);
      },
      () => setInstalledTemplates([]),
    );
    api.listBeamerThemes().then((t) => setInstalledThemeCount(t.themes.length), () => {});
    refreshUserTemplates();
  }, []);

  // Installed-TeX templates not already covered by a built-in one (IEEEtran…), and
  // the class of the active one, if it is one.
  const texTemplates = useMemo(() => {
    const covered = new Set(builtins.map((b) => b.class).filter(Boolean));
    return installedTemplates.filter((t) => !covered.has(t.class));
  }, [builtins, installedTemplates]);
  const currentTexClass = !templateInfo?.path && templateInfo?.template.meta.id.startsWith("tl-") ? templateInfo.template.base?.class ?? null : null;

  // Presentations: slides stacked down the page like the rest of the app.
  const isDeck = template?.kind === "presentation";

  // Node views (references, citations) read the template from the editor.
  const activeTemplate = templateInfo?.template ?? null;
  useEffect(() => {
    if (editor && activeTemplate) setTexloomContext(editor, { template: activeTemplate });
  }, [editor, activeTemplate]);

  const onBibliographyChange = (source: string) => {
    setBibliography(source);
    markDirty();
    scheduleCompile();
  };

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

  /** Returns whether a template was opened. */
  /** Runs a template-loading call; shows template errors in the template panel. */
  const loadTemplateWith = async (load: () => Promise<api.TemplateInfo>): Promise<boolean> => {
    try {
      applyTemplate(await load());
      return true;
    } catch (e) {
      const err = api.asAppError(e);
      if (err.kind === "templateInvalid") setTemplateProblem({ path: err.path, issues: err.issues });
      else report(e);
      return false;
    }
  };

  /** Opens a template file in place (edits to it reload live). Returns whether one was opened. */
  const openTemplateFile = async (path?: string): Promise<boolean> => {
    const target = path ?? (await open({ filters: TEMPLATE_FILTER, multiple: false, directory: false }));
    return target ? loadTemplateWith(() => api.openTemplate(target)) : false;
  };

  /** Imports a template, template package or publisher kit into My templates. */
  const importTemplateFile = async (): Promise<boolean> => {
    const source = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Template, template package or publisher LaTeX kit", extensions: ["json", "zip"] }],
    });
    if (!source) return false;
    const ok = await loadTemplateWith(() => api.importTemplate(source));
    refreshUserTemplates();
    return ok;
  };

  const exportTemplateFile = async () => {
    if (!template) return;
    const target = await save({ filters: [{ name: "Texloom template package", extensions: ["zip"] }], defaultPath: `${template.meta.id}.zip` });
    if (target) await api.exportTemplate(target).catch(report);
  };

  const removeUserTemplate = async (t: api.UserTemplate) => {
    if (!(await ask(`Remove “${t.name}” from My templates? Documents made with it keep a copy of the template.`, { title: "Remove template", kind: "warning" }))) return;
    await api.removeUserTemplate(t.id).catch(report);
    refreshUserTemplates();
  };

  const switchToBuiltin = async (id: string): Promise<boolean> => {
    try {
      applyTemplate(await api.switchToBuiltinTemplate(id));
      return true;
    } catch (e) {
      report(e);
      return false;
    }
  };

  /** Restyles the presentation (theme picker): the template becomes an edited draft,
   *  as in the designer, so it compiles at once and can be saved as a template. */
  const changePresentation = async (patch: Partial<PresentationSettings>, installedOnly: boolean) => {
    const base = draft ?? template;
    if (!base?.presentation) return;
    const next = structuredClone(base);
    next.presentation = { ...base.presentation, ...patch };
    if (next.presentation.accentColor === undefined) delete next.presentation.accentColor;
    onDraftChange(next);
    // Themes only the installed TeX has don't exist in Tectonic's package bundle.
    if (installedOnly && settingsRef.current.engine === "tectonic" && !askedThemeEngine.current) {
      askedThemeEngine.current = true;
      const engines = await api.listEngines().catch(() => []);
      // pdfLaTeX: what the picker's previews of these themes are rendered with.
      const pdf = engines.find((e) => e.id === "pdflatex" && e.available);
      if (pdf && (await ask("This theme comes from your TeX installation; Tectonic, the current engine, doesn't have it.\n\nSwitch the TeX engine to pdfLaTeX from your TeX installation? (Settings → Preview)", { title: "TeX engine", kind: "info", okLabel: "Use pdfLaTeX", cancelLabel: "Keep Tectonic" })))
        updateSettings({ engine: "pdflatex" });
    }
  };

  /** Installed classes compile best with the installed TeX: Tectonic downloads its
   *  own (older) copy of each class, some aren't in it, and a few classes need pdfLaTeX. */
  const switchToInstalled = async (cls: string): Promise<boolean> => {
    const entry = installedTemplates.find((t) => t.class === cls);
    const engine = settingsRef.current.engine;
    if (engine === "tectonic" || (entry?.pdflatexOnly && engine !== "pdflatex")) {
      const engines = await api.listEngines().catch(() => []);
      if (engines.some((e) => e.id === "pdflatex" && e.available)) {
        const why =
          engine === "tectonic"
            ? "This template uses the class installed on your computer. Tectonic, the current engine, downloads its own copy of each class, which may be older or missing."
            : "This class works with pdfLaTeX only.";
        const sw = await ask(`${why}\n\nSwitch the TeX engine to pdfLaTeX from your TeX installation? (Settings → Preview)`, {
          title: "TeX engine",
          kind: "info",
          okLabel: "Use pdfLaTeX",
          cancelLabel: `Keep ${engine === "tectonic" ? "Tectonic" : engine === "xelatex" ? "XeLaTeX" : "LuaLaTeX"}`,
        });
        if (sw) updateSettings({ engine: "pdflatex" });
      }
    }
    try {
      applyTemplate(await api.switchToInstalledTemplate(cls));
      return true;
    } catch (e) {
      report(e);
      return false;
    }
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

  /** Saves `t` under a new name: into My templates (the default), or to a .json file
   *  the user picks ("Save to file…" in the dialog). Null when cancelled. */
  const saveDraftAs = useCallback(
    async (t: api.Template, title?: string): Promise<api.TemplateInfo | null> => {
      // A copy already has its own name; otherwise a name My templates doesn't have yet.
      const name = t.meta.name;
      const defaultName = draftNeedsSaveAs || !userTemplates.some((u) => u.name === name) ? name : `${name} (copy)`;
      const choice = await new Promise<SaveAsChoice | null>((resolve) => setSaveAsRequest({ defaultName, title, resolve }));
      setSaveAsRequest(null);
      if (!choice) return null;
      if (choice.kind === "file") {
        const target = await save({ filters: TEMPLATE_FILTER, defaultPath: `${t.meta.id}.json` });
        return target ? api.saveTemplate(target, t) : null;
      }
      const info = await api.saveTemplateToLibrary(choice.name, t);
      refreshUserTemplates();
      return info;
    },
    [draftNeedsSaveAs, userTemplates, refreshUserTemplates],
  );

  const saveTemplateAs = useCallback(async () => {
    if (!draft) return;
    try {
      const info = await saveDraftAs(draft);
      if (info) applyTemplate(info);
    } catch (e) {
      const err = api.asAppError(e);
      if (err.kind === "templateInvalid") setDraftIssues(err.issues);
      else setStatus({ state: "error", error: err });
    }
  }, [draft, saveDraftAs, applyTemplate]);

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

  /** Imports a publisher kit; a template without its own file is saved first (the kit
   *  lives next to it), and the template is saved again with the kit's files. */
  const importKit = async (zip: string) => {
    if (!draft) return;
    let t = draft;
    let target = templateInfo?.path ?? null;
    if (!target || draftNeedsSaveAs) {
      const info = await saveDraftAs(t, "Save the template first: the kit is stored with it.");
      if (!info?.path) return;
      applyTemplate(info);
      t = info.template;
      target = info.path;
    }
    const next = withKitFiles(t, await api.importKitZip(zip));
    try {
      applyTemplate(await api.saveTemplate(target, next));
    } catch (e) {
      onDraftChange(next); // keep the files in the draft; the error says what to fix
      throw e;
    }
  };

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
    markDirty();
    scheduleCompile();
  };

  // Keep the LaTeX source view current while it's open.
  useEffect(() => {
    if (!showLatex || !editor) return;
    const refresh = () => api.toLatex(docState()).then(setLatex);
    refresh();
    editor.on("update", refresh);
    return () => void editor.off("update", refresh);
  }, [showLatex, editor, metadata]);

  const report = (e: unknown) => setStatus({ state: "error", error: api.asAppError(e) });

  const saveAs = useCallback(async () => {
    if (!editor) return;
    const target = await save({ filters: DOC_FILTER, defaultPath: path ?? inFolder(settingsRef.current.defaultFolder, "report.loom") });
    if (!target) return;
    await api.saveDocument(target, docState()).then(() => {
      setPath(target);
      addRecent(target);
      markClean();
    }, report);
  }, [editor, path, markClean, addRecent, settingsRef]);

  /** Saves the document and its images as one .loom file, to email or upload. */
  const saveAsSingleFile = async () => {
    if (!editor) return;
    const target = await save({
      title: "Save as a single file (images included)",
      filters: DOC_FILTER,
      defaultPath: path ?? inFolder(settingsRef.current.defaultFolder, "report.loom"),
    });
    if (!target) return;
    await api.saveDocument(target, docState(), true).then(() => {
      setPath(target);
      addRecent(target);
      markClean();
    }, report);
  };

  const saveDoc = useCallback(async () => {
    if (!editor) return;
    if (!path) return saveAs();
    await api.saveDocument(path, docState()).then(() => void markClean(), report);
  }, [editor, path, saveAs, markClean]);

  /** Opens a document from the dialog, or `target` (a recent file). */
  const openDoc = async (target?: string) => {
    if (!editor) return;
    if (dirty && !(await ask("Discard unsaved changes and open another document?", { title: "Open document", kind: "warning" }))) return;
    if (!target) {
      const picked = await open({ filters: DOC_FILTER, multiple: false, directory: false, defaultPath: settings.defaultFolder || undefined });
      if (!picked) return;
      target = picked;
    }
    try {
      const opened = await api.openDocument(target);
      addRecent(target);
      loadDocument(editor, opened.doc);
      focusStart(editor, editorScroll.current);
      setBibliography(opened.bibliography ?? "");
      metadataRef.current = opened.metadata;
      setPath(target);
      markClean();
      if (opened.template) applyTemplate(opened.template);
      else {
        setMetadata(opened.metadata);
        compile();
      }
    } catch (e) {
      // A recent file that was moved or deleted leaves the list.
      if (settings.recentFiles.includes(target)) removeRecent(target);
      report(e);
    }
  };

  const exportPdfFile = async () => {
    if (!editor) return;
    const name = (path ? fileNameOf(path).replace(/\.loom$/, "") : "report") + ".pdf";
    const target = await save({ filters: [{ name: "PDF", extensions: ["pdf"] }], defaultPath: path ? name : inFolder(settings.defaultFolder, name) });
    if (target) await api.exportPdf(target, docState()).catch(report);
  };

  const exportLatex = async () => {
    if (!editor) return;
    const target = await save({ filters: [{ name: "LaTeX", extensions: ["tex"] }], defaultPath: inFolder(settings.defaultFolder, "report.tex") });
    if (target) await api.exportTex(target, docState()).catch(report);
  };

  const newDoc = async (skipAsk = false) => {
    if (!editor) return;
    if (!skipAsk && dirty && !(await ask("Discard unsaved changes and start a new document?", { title: "New document", kind: "warning" }))) return;
    try {
      const fresh = await api.newDocument();
      loadDocument(editor, fresh.doc);
      focusStart(editor, editorScroll.current);
      setMetadata({});
      setBibliography("");
      setPath(null);
      markClean();
      setPanel("info"); // a new document starts with its title, authors…
      compile();
    } catch (e) {
      report(e);
    }
  };

  const openGallery = () => {
    setStartWindow(false);
    setShowGallery(true);
  };

  /** The example senior design report, with its template. */
  const openExample = async (which: "report" | "presentation" = "report") => {
    if (!editor) return;
    if (dirty && !(await ask("Discard unsaved changes and open the example?", { title: "Example", kind: "warning" }))) return;
    setShowGallery(false);
    try {
      const deck = which === "presentation";
      const [t, sample] = await Promise.all([
        api.switchToBuiltinTemplate(deck ? "beamer-presentation" : "senior-design"),
        api.sampleDocument(deck ? "presentation" : undefined),
      ]);
      loadDocument(editor, sample.doc);
      focusStart(editor, editorScroll.current);
      setBibliography(sample.bibliography ?? "");
      metadataRef.current = sample.metadata;
      setPath(null);
      markClean();
      applyTemplate(t);
    } catch (e) {
      report(e);
    }
  };

  /** New document with another template: from the gallery, or a template file. */
  const newFromTemplate = async (choose: () => Promise<boolean>, recentKey?: string) => {
    if (dirty && !(await ask("Discard unsaved changes and start a new document?", { title: "New document", kind: "warning" }))) return;
    setShowGallery(false);
    if (!(await choose())) return;
    // "Recently used" in the template window: the last five, newest first.
    if (recentKey) updateSettings((s) => ({ recentTemplates: [recentKey, ...s.recentTemplates.filter((k) => k !== recentKey)].slice(0, 5) }));
    await newDoc(true);
  };

  /** SyncTeX, PDF → editor: puts the cursor in the block behind a double-clicked spot. */
  const jumpFromPdf = async (page: number, x: number, y: number) => {
    const index = await api.pdfToBlock(page, x, y).catch(() => null);
    if (index != null) jumpToBlock(index);
  };

  /** Puts the cursor in top-level block `index` of the document and marks it briefly. */
  const jumpToBlock = (index: number) => {
    if (!editor || index >= editor.state.doc.childCount) return;
    const go = () => {
      let pos = 0;
      for (let i = 0; i < index; i++) pos += editor.state.doc.child(i).nodeSize;
      const { state, view } = editor;
      view.dispatch(state.tr.setSelection(Selection.near(state.doc.resolve(Math.min(pos + 1, state.doc.content.size)))));
      view.focus();
      flashBlock(editor, pos);
      const dom = editor.view.nodeDOM(pos);
      if (dom instanceof HTMLElement) dom.scrollIntoView({ block: "center", behavior: "smooth" });
    };
    // From the designer: show the editor first (it's hidden there), then go.
    if (mode !== "document") {
      setMode("document");
      window.setTimeout(go, 0);
    } else go();
  };

  /** A few words about block `index` for the error list: "Heading “Results”". */
  const describeBlock = (index: number): string => {
    const node = editor?.state.doc.maybeChild(index);
    if (!node) return "";
    const kind: Record<string, string> = {
      heading: "Heading",
      paragraph: "Paragraph",
      figure: "Figure",
      tableFigure: "Table",
      table: "Table",
      equation: "Equation",
      rawLatex: "Raw LaTeX",
      bulletList: "List",
      orderedList: "List",
      blockquote: "Quote",
      codeBlock: "Code",
      slide: "Slide",
    };
    const text = node.textContent.trim().replace(/\s+/g, " ");
    const name = kind[node.type.name] ?? "Block";
    return text ? `${name} “${text.length > 40 ? text.slice(0, 40) + "…" : text}”` : name;
  };

  /** SyncTeX, editor → PDF: scrolls the preview to the block with the cursor. */
  const showInPdf = async () => {
    if (!editor) return;
    const spot = await api.blockToPdf(editor.state.selection.$from.index(0)).catch(() => null);
    if (!spot) return;
    setShowLatex(false);
    setPdfSpot({ ...spot }); // a new object, so showing the same place again works
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey)) return;
      const key = e.key.toLowerCase();
      const run = (f: () => unknown) => {
        e.preventDefault();
        f();
      };
      // Undo/redo reach the document even when the editor lost focus (WebKit drops it
      // when a delete removes the focused part of the page); fields keep their own.
      if ((key === "z" || key === "y") && mode === "document" && editor && !editor.isFocused) {
        const t = e.target as HTMLElement | null;
        if (t?.closest("input, textarea, select, [contenteditable='true']")) return;
        const redo = key === "y" || e.shiftKey;
        run(() => (redo ? editor.chain().focus().redo().run() : editor.chain().focus().undo().run()));
        return;
      }
      if (key === "s") {
        if (mode === "template") run(e.shiftKey ? saveTemplateAs : saveTemplate);
        else run(e.shiftKey ? saveAs : saveDoc);
      } else if (key === "o" && !e.shiftKey) run(() => openDoc());
      else if (key === "n" && e.shiftKey) run(() => openGallery());
      else if (key === "n") run(() => newDoc());
      else if (key === "e" && e.shiftKey) run(exportPdfFile);
      else if (key === "enter") run(compile);
      else if (key === "j" && mode === "document") run(showInPdf);
      else if ((key === "f" || key === "h") && mode === "document" && !e.shiftKey) run(() => setSearchBar({ replace: key === "h" }));
      else if (key === ",") run(() => setShowSettings(true));
    };
    // Capture phase: these shortcuts must win over the editor's own keymap, which runs
    // first otherwise (Ctrl+Enter used to compile *and* insert a line break). The editor
    // skips events that are already handled (defaultPrevented).
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  });

  // The TeX engine follows the setting (and recompiles when it changes).
  const engineReady = useRef(false);
  useEffect(() => {
    api.setEngine(settings.engine).then(
      () => {
        if (engineReady.current) compile();
        engineReady.current = true;
      },
      report,
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [settings.engine]);

  // Spell checking follows the setting.
  useEffect(() => {
    editor?.view.dom.setAttribute("spellcheck", String(settings.spellcheck));
  }, [editor, settings.spellcheck]);

  // Words in the document body, for the status bar.
  const wordCount = useEditorState({
    editor,
    selector: ({ editor }) => (editor ? countWords(editor.state.doc.textBetween(0, editor.state.doc.content.size, " ", " ")) : 0),
  });

  // Presentations: what the slides' footer and logo show on screen.
  const slideCount = useEditorState({
    editor,
    selector: ({ editor }) => {
      let n = 0;
      editor?.state.doc.forEach((node) => node.type.name === "slide" && n++);
      return n;
    },
  });
  const logoPath = typeof metadata.logo === "string" ? metadata.logo : "";
  const { url: logoUrl } = useDocumentImage(isDeck ? logoPath : "");
  const deckInfo: DeckInfo | undefined = (() => {
    const p = template?.presentation;
    if (!isDeck || !p) return undefined;
    const text = (id: string) => (typeof metadata[id] === "string" ? (metadata[id] as string).trim() : "");
    const authors = toAuthors(metadata.authors).map((a) => a.name.trim()).filter(Boolean);
    const leading = (text("title") ? 1 : 0) + (p.outline ? 1 : 0);
    const references = bibliography.trim() && citedKeys(editor).length > 0 ? 1 : 0;
    return {
      title: text("short_title") || text("title"),
      author: authors.join(", "),
      date: text("date") || new Date().toLocaleDateString("en-US", { month: "long", day: "numeric", year: "numeric" }),
      institute: text("institute"),
      pages: leading + (slideCount ?? 0) + references,
      firstNumber: leading,
      logo: !!logoPath && p.logo === "everySlide",
    };
  })();

  const fileName = path ? fileNameOf(path) : "Untitled";
  const missing = issues.filter((i) => i.path.startsWith("metadata.") && i.message.includes("required")).length;
  const headingNames =
    template?.structure.topLevel === "section"
      ? ["Section", "Subsection", "Subsubsection", "Paragraph"]
      : ["Chapter", "Section", "Subsection", "Subsubsection"];
  const draftErrors = draftIssues.filter((i) => i.severity === "error").length;
  const panelTitle: Record<SidePanel, string> = {
    outline: "Outline",
    info: "Document info",
    refs: "References",
    checks: "Things to check",
  };

  return (
    <div className="app">
      <header className="titlebar">
        <div className="titlebar-side">
          <Menu label={<><MenuIcon size={16} /><span>File</span></>} className="tb-button">
            {(close) => {
              const act = (f: () => unknown) => () => {
                close();
                f();
              };
              return (
                <>
                  <MenuItem icon={<FilePlus2 />} label="New" shortcut="Ctrl+N" onClick={act(() => newDoc())} title="New empty document with the current template" />
                  <MenuItem icon={<LayoutTemplate />} label="New from template…" shortcut="Ctrl+Shift+N" onClick={act(() => openGallery())} title="Reports, IEEE, Elsevier, ACM, Springer…" />
                  <MenuItem icon={<FolderOpen />} label="Open…" shortcut="Ctrl+O" onClick={act(() => openDoc())} />
                  <MenuItem icon={<Sparkles />} label="Open the example report" onClick={act(() => openExample())} />
                  <MenuItem icon={<Sparkles />} label="Open the example presentation" onClick={act(() => openExample("presentation"))} />
                  {settings.recentFiles.length > 0 && (
                    <>
                      <MenuHeading>Recent</MenuHeading>
                      {settings.recentFiles.map((f) => (
                        <MenuItem key={f} icon={<History />} label={fileNameOf(f)} title={f} current={f === path} onClick={act(() => openDoc(f))} />
                      ))}
                    </>
                  )}
                  <MenuSeparator />
                  <MenuItem icon={<Save />} label="Save" shortcut="Ctrl+S" onClick={act(saveDoc)} />
                  <MenuItem label="Save as…" shortcut="Ctrl+Shift+S" onClick={act(saveAs)} />
                  <MenuItem
                    label="Save as single file…"
                    onClick={act(saveAsSingleFile)}
                    title="One .loom file with the images inside, to email or upload. Saving it again keeps it one file."
                  />
                  <MenuSeparator />
                  <MenuItem icon={<FileDown />} label="Export PDF…" shortcut="Ctrl+Shift+E" onClick={act(exportPdfFile)} />
                  <MenuItem
                    icon={<FileCode2 />}
                    label="Export LaTeX…"
                    onClick={act(exportLatex)}
                    title="Writes the .tex file, the template's .cls and images next to each other"
                  />
                  <MenuSeparator />
                  <MenuItem icon={<SettingsIcon />} label="Settings" shortcut="Ctrl+," onClick={act(() => setShowSettings(true))} />
                </>
              );
            }}
          </Menu>
          <div className="doc-title" title={path ?? "Not saved yet"}>
            <FileText size={15} aria-hidden="true" />
            <span className="doc-name">{fileName}</span>
            {dirty && <span className="dirty-dot" title="Unsaved changes" />}
          </div>
        </div>
        <div className="mode-switch" role="tablist" aria-label="Mode">
          <button role="tab" aria-selected={mode === "document"} className={mode === "document" ? "active" : ""} onClick={() => setMode("document")}>
            <PenLine size={15} aria-hidden="true" /> Write
          </button>
          <button role="tab" aria-selected={mode === "template"} className={mode === "template" ? "active" : ""} onClick={() => setMode("template")}>
            <LayoutTemplate size={15} aria-hidden="true" /> Design template
            {draftDirty && <span className="dirty-dot" title="Unsaved template changes" />}
            {draftErrors > 0 && <span className="badge">{draftErrors}</span>}
          </button>
        </div>
        <div className="titlebar-side right">
          <Menu
            align="right"
            className="tb-button template-button"
            title={`Template: ${template?.meta.name ?? ""}. ${templateInfo?.path ?? (currentTexClass ? `Class ${currentTexClass} from your TeX installation` : "Built into Texloom")}`}
            label={
              <>
                <LayoutTemplate size={16} aria-hidden="true" />
                <span className="template-name">{template?.meta.name ?? "…"}</span>
                {templateProblem && <span className="badge">!</span>}
                <ChevronDown size={14} aria-hidden="true" />
              </>
            }
          >
            {(close) => (
            <>
              <div className="menu-info">
                <strong>{template?.meta.name}</strong>
                <span className="muted">
                  {templateInfo?.path ??
                    (currentTexClass ? `Class ${currentTexClass} from your TeX installation` : "Built into Texloom")}
                </span>
                {templateInfo?.path && <span className="muted">Reloads automatically when the file changes.</span>}
                <span className="muted">Choosing another template restyles this document; its text stays.</span>
              </div>
              <MenuItem
                icon={<FilePlus2 />}
                label="New document from template…"
                onClick={() => {
                  close();
                  openGallery();
                }}
              />
              <MenuItem
                icon={<FileInput />}
                label="Import template…"
                title="A Texloom template (.json), a template package (.zip), or any publisher's LaTeX kit (.zip)"
                onClick={() => {
                  close();
                  importTemplateFile();
                }}
              />
              <MenuItem
                icon={<FileOutput />}
                label="Export template…"
                title="Save this template and its files as a .zip others can import"
                onClick={() => {
                  close();
                  exportTemplateFile();
                }}
              />
              <MenuItem
                icon={<FolderOpen />}
                label="Open template file…"
                title="Open a template.json in place; edits to the file reload live"
                onClick={() => {
                  close();
                  openTemplateFile();
                }}
              />
              {userTemplates.length > 0 && (
                <div>
                  <MenuHeading>My templates</MenuHeading>
                  {userTemplates.map((u) => (
                    <MenuItem
                      key={u.id}
                      current={templateInfo?.path === u.path}
                      icon={templateInfo?.path === u.path ? <CircleCheck /> : undefined}
                      label={u.name}
                      shortcut={u.class ?? undefined}
                      title={u.description}
                      onClick={() => {
                        close();
                        openTemplateFile(u.path);
                      }}
                    />
                  ))}
                </div>
              )}
              {[
                { title: "Reports", items: builtins.filter((b) => !b.class && b.kind !== "presentation") },
                { title: "Presentations", items: builtins.filter((b) => b.kind === "presentation") },
                { title: "Journals and conferences", items: builtins.filter((b) => b.class) },
              ].map(
                (group) =>
                  group.items.length > 0 && (
                    <div key={group.title}>
                      <MenuHeading>{group.title}</MenuHeading>
                      {group.items.map((b) => (
                        <MenuItem
                          key={b.id}
                          current={!templateInfo?.path && template?.meta.id === b.id}
                          icon={!templateInfo?.path && template?.meta.id === b.id ? <CircleCheck /> : undefined}
                          label={b.name}
                          shortcut={b.class ?? undefined}
                          title={b.description}
                          onClick={() => {
                            close();
                            switchToBuiltin(b.id);
                          }}
                        />
                      ))}
                    </div>
                  ),
              )}
              {texTemplates.length > 0 && (
                <div>
                  <MenuHeading>From your TeX installation</MenuHeading>
                  {currentTexClass && (
                    <MenuItem current icon={<CircleCheck />} label={template?.meta.name ?? currentTexClass} shortcut={currentTexClass} onClick={close} />
                  )}
                  <MenuItem
                    icon={<Library />}
                    label={`Choose from ${texTemplates.length} classes…`}
                    title="Journal, thesis, book and article classes found in your TeX distribution"
                    onClick={() => {
                      close();
                      setShowTexPicker(true);
                    }}
                  />
                </div>
              )}
            </>
          )}
          </Menu>
          <button className="tb-button" onClick={exportPdfFile} title="Export PDF (Ctrl+Shift+E)">
            <FileDown size={16} aria-hidden="true" /> Export PDF
          </button>
          <button className="icon-button" onClick={() => setShowSettings(true)} title="Settings (Ctrl+,)" aria-label="Settings">
            <SettingsIcon size={17} />
          </button>
        </div>
      </header>

      {editor && mode === "document" && <FormatBar editor={editor} headingNames={headingNames} onError={report} onPicker={setPicker} deck={isDeck} onThemes={() => setShowThemePicker(true)} />}

      <main className={`workspace${mode === "template" ? " template-mode" : ""}`}>
        {mode === "document" && (
          <nav className="rail" aria-label="Panels">
            <RailButton icon={<ListTree />} label="Outline" active={panel === "outline"} onClick={() => togglePanel("outline")} />
            <RailButton icon={<IdCard />} label="Document info" badge={missing || undefined} active={panel === "info"} onClick={() => togglePanel("info")} />
            <RailButton
              icon={<Library />}
              label="References"
              badge={bibInfo.error ? "!" : undefined}
              active={panel === "refs"}
              onClick={() => togglePanel("refs")}
            />
            <RailButton
              icon={<TriangleAlert />}
              label="Things to check"
              badge={issues.length || undefined}
              warn
              active={panel === "checks"}
              onClick={() => togglePanel("checks")}
            />
          </nav>
        )}
        {mode === "document" && panel && (
          <aside className={`side-panel side-${panel}`} aria-label={panelTitle[panel]}>
            <div className="side-head">
              <span>{panelTitle[panel]}</span>
              <button className="icon-button small" onClick={() => setPanel(null)} aria-label="Close panel" title="Close">
                <X size={15} />
              </button>
            </div>
            <div className="side-body">
              {panel === "outline" && editor && template && <Outline editor={editor} />}
              {panel === "info" && template && <MetadataForm template={template} metadata={metadata} onChange={onMetadataChange} />}
              {panel === "refs" && <ReferencesPanel source={bibliography} info={bibInfo} cited={citedKeys(editor)} onChange={onBibliographyChange} />}
              {panel === "checks" && <ChecksList issues={issues} />}
            </div>
          </aside>
        )}
        {mode === "template" && templateInfo && draft && (
          <section className="designer-pane">
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
              onImportKit={importKit}
              onBrowseThemes={() => setShowThemePicker(true)}
            />
          </section>
        )}
        {/* Kept mounted (just hidden) in template mode so the editor keeps its state. */}
        <section className="editor-pane" hidden={mode !== "document"}>
          {template && <style>{templateCss(template, ".editor-pane .page", deckInfo)}</style>}
          {search && editor && <SearchBar editor={editor} withReplace={search.replace} onClose={() => setSearchBar(null)} />}
          {picker && editor && (
            <div className="picker-anchor">
              {picker === "ref" ? (
                <RefPicker editor={editor} onClose={() => setPicker(null)} />
              ) : (
                <CitePicker editor={editor} entries={bibInfo.entries} onClose={() => setPicker(null)} onManage={() => setPanel("refs")} />
              )}
            </div>
          )}
          <div className="editor-scroll" ref={editorScroll} style={{ ["--page-zoom" as string]: pageZoom }}>
            <div className={`page${isDeck ? " deck" : ""}`} style={logoUrl ? { ["--slide-logo" as string]: `url("${logoUrl}")` } : undefined}>
              <EditorContent editor={editor} />
            </div>
          </div>
        </section>
        <section className="preview-pane">
          <div className="pane-head">
            <span className="pane-title">{showLatex ? "LaTeX source" : "Preview"}</span>
            {mode === "template" && !showLatex && (
              <span className="segmented" role="group" aria-label="Preview kind">
                <button className={designPreview === "pdf" ? "active" : ""} aria-pressed={designPreview === "pdf"} onClick={() => setDesignPreview("pdf")} title="The compiled PDF (exact)">
                  PDF
                </button>
                <button className={designPreview === "page" ? "active" : ""} aria-pressed={designPreview === "page"} onClick={() => setDesignPreview("page")} title="The document's page with this template, updated instantly (no compile)">
                  Page
                </button>
              </span>
            )}
            <span className="spacer" />
            {!settings.autoCompile && (
              <button className="tb-button" onClick={compile} title="Compile now (Ctrl+Enter)">
                <Play size={14} aria-hidden="true" /> Compile
              </button>
            )}
            {!showLatex && (
              <span className="zoom" role="group" aria-label="Preview zoom">
                <button className="icon-button small" onClick={() => setPreviewZoom((z) => Math.max(50, z - 10))} aria-label="Zoom out" title="Zoom out">
                  <ZoomOut size={15} />
                </button>
                <button className="zoom-value" onClick={() => setPreviewZoom(100)} title="Fit width">
                  {previewZoom}%
                </button>
                <button className="icon-button small" onClick={() => setPreviewZoom((z) => Math.min(200, z + 10))} aria-label="Zoom in" title="Zoom in">
                  <ZoomIn size={15} />
                </button>
              </span>
            )}
            {mode === "document" && (
              <button
                className="icon-button small"
                onClick={showInPdf}
                disabled={!pdf}
                aria-label="Show in PDF"
                title="Show the cursor's place in the PDF (Ctrl+J). Double-click the PDF to go back to the editor."
              >
                <LocateFixed size={15} />
              </button>
            )}
            <button
              className={`icon-button small${showLatex ? " active" : ""}`}
              onClick={() => setShowLatex((v) => !v)}
              aria-label="LaTeX source"
              title={showLatex ? "Back to the PDF" : "Show the generated LaTeX"}
            >
              <Code size={15} />
            </button>
          </div>
          {templateProblem && <TemplateProblemPanel problem={templateProblem} onDismiss={() => setTemplateProblem(null)} />}
          {status.state === "error" && <ErrorPanel error={status.error} onJump={jumpToBlock} describe={describeBlock} />}
          {showLatex ? (
            <pre className="latex-source">{latex}</pre>
          ) : mode === "template" && designPreview === "page" && draft ? (
            <PagePreview editor={editor} template={draft} deck={deckInfo} />
          ) : (
            <PdfPreview pdf={pdf} zoom={previewZoom} onPages={setPageCount} onLocate={mode === "document" ? jumpFromPdf : undefined} spot={pdfSpot} />
          )}
        </section>
      </main>

      <footer className="statusbar">
        <StatusBadge status={status} ms={compileMs} />
        {pageCount > 0 && <span>{pageCount} page{pageCount === 1 ? "" : "s"}</span>}
        {mode === "document" && <span>{wordCount.toLocaleString()} words</span>}
        {mode === "document" && issues.length > 0 && (
          <button className="status-link warn" onClick={() => setPanel("checks")}>
            <TriangleAlert size={13} aria-hidden="true" /> {issues.length} to check
          </button>
        )}
        <span className="spacer" />
        {mode === "document" && template && (
          <span title={templateInfo?.path ?? "Built-in template"}>
            <LayoutTemplate size={13} aria-hidden="true" /> {template.meta.name}
          </span>
        )}
        {mode === "document" && (
          <Menu up align="right" className="status-link" title="Page zoom" label={<>Page {settings.pageZoom ? `${settings.pageZoom}%` : `fit (${Math.round(fitZoom * 100)}%)`}</>}>
            {(close) =>
              [0, 75, 90, 100, 110, 125, 150].map((z) => (
                <MenuItem
                  key={z}
                  current={settings.pageZoom === z}
                  icon={settings.pageZoom === z ? <CircleCheck /> : undefined}
                  label={z ? `${z}%` : "Fit width"}
                  onClick={() => {
                    close();
                    updateSettings({ pageZoom: z });
                  }}
                />
              ))
            }
          </Menu>
        )}
      </footer>

      {saveAsRequest && (
        <SaveTemplateDialog
          defaultName={saveAsRequest.defaultName}
          title={saveAsRequest.title}
          taken={userTemplates.map((u) => u.name)}
          onDone={saveAsRequest.resolve}
        />
      )}
      {showGallery && (
        <NewDocumentDialog
          builtins={builtins}
          currentId={templateInfo?.path ? null : template?.meta.id ?? null}
          onPick={(id) => newFromTemplate(() => switchToBuiltin(id), `builtin:${id}`)}
          userTemplates={userTemplates}
          currentPath={templateInfo?.path ?? null}
          onPickUser={(u) => newFromTemplate(() => openTemplateFile(u.path), `mine:${u.path}`)}
          onRemoveUser={removeUserTemplate}
          onImport={() => newFromTemplate(importTemplateFile)}
          installed={installedTemplates}
          waiting={installedWaiting}
          installedThemes={installedThemeCount}
          currentClass={currentTexClass}
          onPickInstalled={(c) => newFromTemplate(() => switchToInstalled(c), `tex:${c}`)}
          engine={settings.engine}
          start={startWindow}
          view={settings.galleryView}
          category={settings.galleryCategory}
          recentTemplates={settings.recentTemplates}
          onPrefs={(p) => updateSettings(p)}
          recentFiles={settings.recentFiles}
          onOpen={(p) => {
            setShowGallery(false);
            openDoc(p);
          }}
          onExample={() => openExample()}
          onExamplePresentation={() => openExample("presentation")}
          onClose={() => {
            setShowGallery(false);
            // Closing the start window starts writing in the empty document.
            if (startWindow && editor) focusStart(editor, editorScroll.current);
            setStartWindow(false);
          }}
        />
      )}
      {showThemePicker && (draft ?? template)?.presentation && (
        <ThemePicker
          settings={(draft ?? template)!.presentation!}
          engine={settings.engine}
          onChange={changePresentation}
          onClose={() => setShowThemePicker(false)}
        />
      )}
      {showTexPicker && (
        <NewDocumentDialog
          only="installed"
          builtins={builtins}
          currentId={templateInfo?.path ? null : template?.meta.id ?? null}
          onPick={(id) => {
            setShowTexPicker(false);
            switchToBuiltin(id);
          }}
          userTemplates={[]}
          currentPath={null}
          onPickUser={() => {}}
          onRemoveUser={() => {}}
          onImport={() => {}}
          installed={installedTemplates}
          waiting={installedWaiting}
          installedThemes={installedThemeCount}
          currentClass={currentTexClass}
          onPickInstalled={(c) => {
            setShowTexPicker(false);
            switchToInstalled(c);
          }}
          engine={settings.engine}
          onClose={() => setShowTexPicker(false)}
        />
      )}
      {showSettings && <SettingsDialog settings={settings} onChange={updateSettings} onClose={() => setShowSettings(false)} />}
    </div>
  );
}

function RailButton({
  icon,
  label,
  active,
  badge,
  warn,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  active: boolean;
  badge?: number | string;
  warn?: boolean;
  onClick: () => void;
}) {
  return (
    <button className={`rail-button${active ? " active" : ""}`} onClick={onClick} title={label} aria-label={label} aria-pressed={active}>
      {icon}
      {badge !== undefined && <span className={`rail-badge${warn ? " warn" : ""}`}>{badge}</span>}
    </button>
  );
}

function ChecksList({ issues }: { issues: api.Issue[] }) {
  if (issues.length === 0) {
    return (
      <p className="side-empty">
        <CircleCheck size={16} aria-hidden="true" /> Nothing to check: required fields are filled in and every reference
        points somewhere.
      </p>
    );
  }
  return (
    <ul className="checks-list">
      {issues.map((i, n) => (
        <li key={n}>
          <TriangleAlert size={14} aria-hidden="true" />
          <span>{i.message}</span>
        </li>
      ))}
    </ul>
  );
}

function FormatBar({
  editor,
  headingNames,
  onError,
  onPicker,
  deck,
  onThemes,
}: {
  editor: Editor;
  headingNames: string[];
  onError: (e: unknown) => void;
  onPicker: (p: "ref" | "cite") => void;
  /** A presentation: slide tools instead of article-only ones. */
  deck: boolean;
  /** Opens the presentation theme picker. */
  onThemes: () => void;
}) {
  // Re-render only when the formatting state at the cursor changes.
  const s = useEditorState({
    editor,
    selector: ({ editor }) => ({
      bold: editor.isActive("bold"),
      italic: editor.isActive("italic"),
      code: editor.isActive("code"),
      level: editor.isActive("heading") ? (editor.getAttributes("heading").level as number) : 0,
      paragraph: editor.isActive("paragraph"),
      bullet: editor.isActive("bulletList"),
      ordered: editor.isActive("orderedList"),
      quote: editor.isActive("blockquote"),
      inTable: editor.isActive("table"),
      canMerge: editor.can().mergeCells(),
      canSplit: editor.can().splitCell(),
      headingNumbered: editor.getAttributes("heading").numbered !== false,
      canUndo: editor.can().undo(),
      canRedo: editor.can().redo(),
    }),
  });
  const c = () => editor.chain().focus();
  // Blocks can't go in a slide's title: inserts start on the line below it.
  const ins = () => (deck ? c().leaveSlideTitle() : c());
  const T = ({ icon, label, title, on, run, disabled }: { icon: React.ReactNode; label?: string; title: string; on?: boolean; run: () => void; disabled?: boolean }) => (
    <button
      className={`tool${on ? " on" : ""}${label ? " with-label" : ""}`}
      title={title}
      aria-label={label ? undefined : title}
      aria-pressed={on}
      disabled={disabled}
      onMouseDown={(e) => e.preventDefault()}
      onClick={run}
    >
      {icon}
      {label && <span>{label}</span>}
    </button>
  );
  const styleValue = s.level ? `h${s.level}` : s.paragraph ? "p" : "";
  const setStyle = (v: string) => {
    if (v === "p") c().setParagraph().run();
    else c().setHeading({ level: Number(v.slice(1)) as 1 | 2 | 3 | 4 }).run();
  };
  const sz = 17;
  return (
    <div className="toolbar" role="toolbar" aria-label="Formatting">
      <T icon={<Undo2 size={sz} />} title="Undo (Ctrl+Z)" run={() => c().undo().run()} disabled={!s.canUndo} />
      <T icon={<Redo2 size={sz} />} title="Redo (Ctrl+Shift+Z)" run={() => c().redo().run()} disabled={!s.canRedo} />
      <span className="sep" />
      <select className="style-select" value={styleValue} onChange={(e) => setStyle(e.target.value)} title="Paragraph style" aria-label="Paragraph style">
        {!styleValue && <option value="">—</option>}
        <option value="p">Body text</option>
        {headingNames.map((n, i) => (
          <option key={n} value={`h${i + 1}`}>
            {n}
          </option>
        ))}
      </select>
      <span className="sep" />
      <T icon={<Bold size={sz} />} title="Bold (Ctrl+B)" on={s.bold} run={() => c().toggleBold().run()} />
      <T icon={<Italic size={sz} />} title="Italic (Ctrl+I)" on={s.italic} run={() => c().toggleItalic().run()} />
      <T icon={<Code size={sz} />} title="Code (Ctrl+E)" on={s.code} run={() => c().toggleCode().run()} />
      <span className="sep" />
      <T icon={<List size={sz} />} title="Bullet list" on={s.bullet} run={() => ins().toggleBulletList().run()} />
      <T icon={<ListOrdered size={sz} />} title="Numbered list" on={s.ordered} run={() => ins().toggleOrderedList().run()} />
      <T icon={<TextQuote size={sz} />} title="Quote" on={s.quote} run={() => c().toggleBlockquote().run()} />
      <span className="sep" />
      <T icon={<Superscript size={sz} />} title="Inline math (Ctrl+M, or type $x^2$)" run={() => c().insertInlineMath().run()} />
      <T icon={<Sigma size={sz} />} title="Equation on its own line ($$ + space)" run={() => ins().insertEquation().run()} />
      <T
        icon={<Image size={sz} />}
        title="Figure: an image with a caption"
        run={() =>
          pickFigureImage().then((src) => {
            if (src) ins().insertFigure(src).run();
          }, onError)
        }
      />
      <T icon={<Table size={sz} />} title="Table with a caption" run={() => ins().insertCaptionedTable(3, 3).run()} />
      <T icon={<Braces size={sz} />} title="Raw LaTeX block, written into the PDF as typed (TikZ, special tables…)" run={() => ins().insertRawLatex().run()} />
      {deck && (
        <>
          <span className="sep" />
          <T icon={<GalleryVerticalEnd size={sz} />} label="Slide" title="New slide after this one" run={() => c().insertSlide().run()} />
          <T icon={<Columns2 size={sz} />} title="Two columns side by side" run={() => ins().insertColumns(2).run()} />
          <T icon={<SquareStack size={sz} />} title="Box with a title (block; switch to alert or example in its corner)" run={() => ins().insertSlideBox("block").run()} />
          <T icon={<Pause size={sz} />} title="Pause: what follows appears on the next click" run={() => ins().insertPause().run()} />
          <T icon={<Palette size={sz} />} label="Theme" title="Presentation theme and colours" run={onThemes} />
        </>
      )}
      <span className="sep" />
      <T icon={<Link2 size={sz} />} title="Cross-reference (“see Figure 3.1”)" run={() => onPicker("ref")} />
      <T icon={<Quote size={sz} />} title="Citation" run={() => onPicker("cite")} />
      {!deck && <T icon={<BookMarked size={sz} />} title="Start the appendices here (chapters after it are A, B…)" run={() => c().insertAppendices().run()} />}
      <span className="spacer" />
      {s.level > 0 && (
        <span className="context-tools" role="group" aria-label="Heading">
          <T
            icon={<Hash size={15} />}
            label="Numbered"
            on={s.headingNumbered}
            title="Number this heading. Turn off for Acknowledgment, Preface… (still listed in the table of contents)"
            run={() => c().toggleHeadingNumbered().run()}
          />
        </span>
      )}
      {s.inTable && (
        <span className="context-tools" role="group" aria-label="Table">
          <T icon={<Rows3 size={15} />} label="+ Row" title="Add a row below" run={() => c().addRowAfter().run()} />
          <T icon={<Columns3 size={15} />} label="+ Col" title="Add a column to the right" run={() => c().addColumnAfter().run()} />
          <T icon={<Rows3 size={15} />} label="− Row" title="Delete this row" run={() => c().deleteRow().run()} />
          <T icon={<Columns3 size={15} />} label="− Col" title="Delete this column" run={() => c().deleteColumn().run()} />
          <T icon={<Table size={15} />} label="Header" title="Toggle the header row" run={() => c().toggleHeaderRow().run()} />
          <T icon={<TableCellsMerge size={15} />} label="Merge" title="Merge the selected cells into one (drag across cells to select them); it can span rows and columns" run={() => c().mergeCells().run()} disabled={!s.canMerge} />
          <T icon={<TableCellsSplit size={15} />} label="Split" title="Split a merged cell back into single cells" run={() => c().splitCell().run()} disabled={!s.canSplit} />
          <T icon={<Trash2 size={15} />} title="Delete the table and its caption" run={() => c().deleteCaptionedTable().run()} />
        </span>
      )}
    </div>
  );
}

function StatusBadge({ status, ms }: { status: Status; ms: number | null }) {
  if (status.state === "compiling")
    return (
      <span className="status status-compiling">
        <LoaderCircle size={13} className="spin" aria-hidden="true" /> Compiling…
      </span>
    );
  if (status.state === "error")
    return (
      <span className="status status-error">
        <CircleAlert size={13} aria-hidden="true" />{" "}
        {status.error.kind === "texErrors" ? `${status.error.errors.length} LaTeX error${status.error.errors.length === 1 ? "" : "s"}` : "Error"}
      </span>
    );
  if (status.state === "ok")
    return (
      <span className="status status-ok">
        <CircleCheck size={13} aria-hidden="true" /> Up to date{ms != null && ` · ${(ms / 1000).toFixed(1)} s`}
      </span>
    );
  return <span className="status">Ready</span>;
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

/** What a TeX error usually means, for people who don't read LaTeX logs. */
function errorHint(message: string): string | null {
  const m = message.toLowerCase();
  if (m.includes("undefined control sequence")) return "A LaTeX command that doesn't exist: check formulas and raw LaTeX blocks for a typo.";
  if (m.includes("missing $ inserted")) return "A maths symbol (^, _, \\alpha…) outside a formula: put it in a formula, or check raw LaTeX.";
  if (m.includes("missing } inserted") || m.includes("extra }") || m.includes("runaway argument")) return "Unbalanced braces { } in a formula or raw LaTeX block.";
  if (m.includes("not found") && (m.includes("file") || m.includes(".png") || m.includes(".jpg"))) return "A file is missing: an image moved or a package isn't installed.";
  if (m.includes("misplaced alignment tab")) return "An & outside a table or aligned formula.";
  if (m.includes("there's no line here to end")) return "A line break (\\\\) where nothing came before it.";
  return null;
}

function ErrorPanel({ error, onJump, describe }: { error: AppError; onJump?: (block: number) => void; describe?: (block: number) => string }) {
  if (error.kind === "texErrors") {
    return (
      <div className="error-panel">
        <strong>LaTeX reported errors (the preview shows the last good version):</strong>
        <ul className="error-list">
          {error.errors.map((e, i) => {
            const hint = errorHint(e.message);
            const body = (
              <>
                <span className="error-message">{e.message}</span>
                {hint && <span className="error-hint">{hint}</span>}
                {e.block != null && describe && <span className="error-where">in {describe(e.block)}</span>}
              </>
            );
            return (
              <li key={i}>
                {e.block != null && onJump ? (
                  <button type="button" className="error-jump" onClick={() => onJump(e.block!)} title="Go to this place in the editor">
                    {body}
                  </button>
                ) : (
                  <div className="error-item">
                    {e.line != null && <span className="muted">line {e.line}: </span>}
                    {body}
                  </div>
                )}
              </li>
            );
          })}
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
      <strong>{error.kind === "tectonicMissing" ? "TeX engine not available" : "Error"}:</strong> {error.message}
    </div>
  );
}

/** Keys cited anywhere in the document, in order of first use. */
function citedKeys(editor: Editor | null): string[] {
  const keys: string[] = [];
  editor?.state.doc.descendants((n) => {
    if (n.type.name === "citation") for (const k of n.attrs.keys as string[]) if (!keys.includes(k)) keys.push(k);
  });
  return keys;
}
