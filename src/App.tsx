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
import { ChaptersPanel } from "./editor/ChaptersPanel";
import { bookNumbering } from "./editor/numbering";
import { AppendixStart, Citation, CrossRef, TexloomIds, setTexloomContext } from "./editor/Academic";
import { CitePicker, ReferencesPanel, RefPicker } from "./editor/Pickers";
import { PAGE_WIDTH_PX, templateCss, type DeckInfo } from "./editor/templateStyle";
import { useDocumentImage } from "./editor/Figures";
import { toAuthors } from "./editor/AuthorsEditor";
import { DeckDocument, SLIDE_EXTENSIONS } from "./editor/Slides";
import { Spaces } from "./editor/Spaces";
import { Search } from "./editor/Search";
import { Direction } from "./editor/Direction";
import { ArabicPunctuation } from "./editor/ArabicPunctuation";
import { docLanguage } from "./editor/language";
import { setMathSettings } from "./editor/arabicMath";
import { arabicFormulaPictures, formulasAsMathMl } from "./editor/wordExport";
import { ensureLibraryFont, libraryFontIds, templateFonts } from "./editor/libraryFonts";
import { LanguageContext, resolveUiLanguage, translate, useT, t as tNow, type MessageKey } from "./i18n";
import { Flash, flashBlock } from "./editor/Flash";
import { SearchBar } from "./editor/SearchBar";
import { TemplateDesigner } from "./designer/TemplateDesigner";
import { SaveTemplateDialog, type SaveAsChoice } from "./designer/SaveTemplateDialog";
import { NewTemplateDialog } from "./designer/NewTemplateDialog";
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
  BookMarked, Bold, FileInput, FileOutput, Sparkles, Braces, ChevronDown, CircleAlert, CircleCheck, Code, Copy, Check, Columns3, FileCode2, FileDown, FilePlus2,
  FileText, FolderOpen, Hash, History, IdCard, Image, Italic, LayoutTemplate, Library, Link2, List, ListOrdered, ListTree,
  LoaderCircle, Menu as MenuIcon, PenLine, Play, Quote, Redo2, Rows3, Save, Settings as SettingsIcon, Palette, Columns2, SquareStack, Pause, GalleryVerticalEnd, Sigma, Superscript,
  Table, TextQuote, Trash2, TriangleAlert, Undo2, X, ZoomIn, ZoomOut, LocateFixed, TableCellsMerge, TableCellsSplit, ArrowRightLeft, AlignLeft, AlignCenter, AlignRight, Files, Split, Merge,
} from "lucide-react";
import "./App.css";

/** Designer edits are validated after this pause, then the preview recompiles. */
const TEMPLATE_DEBOUNCE_MS = 400;
/** Unsaved changes are copied for crash recovery after this pause in typing. */
const RECOVERY_DELAY_MS = 3000;
// Functions, not constants: the filter names follow the interface language.
const docFilter = () => [{ name: tNow("app.filterDocument"), extensions: ["loom"] }];
/** Opening: documents and projects. */
const openFilter = () => [{ name: tNow("project.filterAll"), extensions: ["loom", "loomproj"] }];
const templateFilter = () => [{ name: tNow("app.filterTemplate"), extensions: ["json"] }];

const EMPTY_DOC: JSONContent = { type: "doc", content: [{ type: "paragraph" }] };

/** Loads a document into the editor; throws a readable error instead of silently
 *  showing an empty page when the document doesn't fit the editor's schema. */
function loadDocument(editor: Editor, doc: JSONContent) {
  try {
    editor.schema.nodeFromJSON(doc).check();
  } catch (e) {
    throw new Error(tNow("app.cantOpenDocument", { error: e instanceof Error ? e.message : String(e) }));
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
    ? tNow("app.todayAt", { time })
    : tNow("app.onDateAt", { date: d.toLocaleDateString([], { day: "numeric", month: "short" }), time });
}

/** `name` inside `folder` (or just `name` without a folder), for dialog defaults. */
const inFolder = (folder: string, name: string) => (folder ? `${folder.replace(/[\\/]+$/, "")}/${name}` : name);

const countWords = (text: string) => (text.match(/[\p{L}\p{N}]+(?:['’-][\p{L}\p{N}]+)*/gu) ?? []).length;

type SidePanel = "chapters" | "outline" | "info" | "refs" | "checks";

/** The open project (a book as one file per chapter), when one is open. */
interface OpenProject {
  /** The .loomproj file. */
  path: string;
  entries: api.ProjectEntry[];
  index: api.ProjectIndex;
  /** The chapter open in the editor. */
  chapter: string;
  /** Problems found when the project was opened (unreadable chapters…). */
  issues: api.Issue[];
}

/** `file` (absolute) relative to `folder`, with forward slashes, if it's inside. */
function relativeTo(folder: string, file: string): string | null {
  const norm = (p: string) => p.replace(/\\/g, "/").replace(/\/+$/, "");
  const f = norm(folder);
  const p = norm(file);
  return p.toLowerCase().startsWith(f.toLowerCase() + "/") ? p.slice(f.length + 1) : null;
}

type Status = { state: "idle" | "compiling" | "ok" } | { state: "error"; error: AppError };

export default function App() {
  const [pdf, setPdf] = useState<ArrayBuffer | null>(null);
  const [status, setStatus] = useState<Status>({ state: "idle" });
  const [path, setPath] = useState<string | null>(null);
  /** The open project; null while a single document is open. */
  const [project, setProjectState] = useState<OpenProject | null>(null);
  const projectRef = useRef<OpenProject | null>(null);
  const setProject = (p: OpenProject | null) => {
    projectRef.current = p;
    setProjectState(p);
  };
  /** Compile the whole book instead of just the open chapter. */
  const [wholeBook, setWholeBookState] = useState(false);
  const wholeBookRef = useRef(false);
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
  // Counts the documents shown (new, opened, example, project): the PDF preview
  // starts each at its first page. Switching chapters keeps the place.
  const [docKey, setDocKey] = useState(0);
  const showDocument = (editor: Editor, doc: JSONContent) => {
    loadDocument(editor, doc);
    setDocKey((k) => k + 1);
  };
  // The template window is choosing the template of a new project.
  const [projectPick, setProjectPick] = useState(false);
  // The gallery as the start window: also offers Open, recent files and the example.
  const [startWindow, setStartWindow] = useState(false);
  const { settings, loaded: settingsLoaded, update: updateSettings, addRecent, removeRecent, latest: settingsRef } = useSettings();
  const [compileMs, setCompileMs] = useState<number | null>(null);
  const [pageCount, setPageCount] = useState(0);
  const [previewZoom, setPreviewZoom] = useState(100);
  const [latex, setLatex] = useState("");
  // "Copied" on the LaTeX view's copy button, for a moment.
  const [latexCopied, setLatexCopied] = useState(false);
  const copyLatex = async () => {
    try {
      await navigator.clipboard.writeText(latex);
    } catch {
      // No clipboard permission in this webview: the old way, through a selection.
      const area = document.createElement("textarea");
      area.value = latex;
      area.style.position = "fixed";
      area.style.opacity = "0";
      document.body.appendChild(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
    setLatexCopied(true);
    window.setTimeout(() => setLatexCopied(false), 1500);
  };
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
  /** The "New blank template" dialog is open. */
  const [showNewTemplate, setShowNewTemplate] = useState(false);
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
  const shownZoom = Math.round(pageZoom * 100);
  // App renders the LanguageContext provider, so its own body can't use useT().
  const uiLang = resolveUiLanguage(settings.uiLanguage);
  const tr = (key: MessageKey, vars?: Record<string, string | number>) => translate(uiLang, key, vars);

  const compile = useCallback(async () => {
    const editor = editorRef.current;
    if (!editor) return;
    const id = ++requestId.current;
    setStatus({ state: "compiling" });
    const t0 = performance.now();
    const state: api.DocState = { doc: editor.getJSON(), metadata: metadataRef.current, bibliography: bibRef.current };
    const p = projectRef.current;
    api.checkDocument(state).then(
      (found) =>
        setIssues(
          p
            ? // A chapter alone: references to other chapters aren't broken, and the
              // template's required chapters are elsewhere in the book.
              [...p.issues, ...found.filter((i) => i.path !== "document.refs" && i.path !== "document.skeleton")]
            : found,
        ),
      () => setIssues([]),
    );
    try {
      let bytes: ArrayBuffer;
      if (p) {
        const s: api.ProjectState = { file: p.chapter, ...state };
        // The index follows the open chapter's live content (numbers in later chapters).
        api.projectIndex(p.chapter, state.doc).then((index) => {
          const cur = projectRef.current;
          if (cur && cur.chapter === p.chapter) setProject({ ...cur, index });
        }, () => {});
        bytes = await api.renderProject(s, !wholeBookRef.current);
      } else bytes = await api.renderPdf(state);
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
      // In a project: the open chapter (switching chapters saves), with the project's
      // metadata and bibliography; restored into the project.
      if (dirtyRef.current && settingsRef.current.crashRecovery) queueRecovery(() => api.writeRecovery(pathRef.current, docState(), projectRef.current?.chapter));
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
      Direction,
      ArabicPunctuation,
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
  // window (templates, open, recent files) over an empty document. Waits for the
  // settings, so the first dialogs use the interface language.
  useEffect(() => {
    if (!editor || !settingsLoaded || started.current) return;
    started.current = true; // React StrictMode runs effects twice in development
    (async () => {
      const t = await api.getTemplate();
      setTemplateInfo(t);
      setDraft(t.template);
      if (await offerRecovery(editor)) return;
      const blank = await api.newDocument();
      showDocument(editor, blank.doc);
      setPanel("info"); // like any new document: start with its title, authors…
      compile();
      setStartWindow(true);
      setShowGallery(true);
    })().catch((e) => setStatus({ state: "error", error: api.asAppError(e) }));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor, settingsLoaded, compile, setMetadata, setBibliography]);

  /** Restores the changes a crash left behind, if the user wants them. */
  const offerRecovery = async (editor: Editor): Promise<boolean> => {
    const info = await api.recoveryInfo().catch(() => null);
    if (!info) return false;
    const name = info.originalPath ? `“${fileNameOf(info.originalPath)}”` : tNow("app.anUnsavedDocument");
    const when = info.savedAt ? tNow("app.lastChange", { when: whenText(info.savedAt) }) : "";
    const restore = await ask(tNow("app.restorePrompt", { name, when }), {
      title: tNow("app.restoreTitle"),
      kind: "warning",
      okLabel: tNow("app.restore"),
      cancelLabel: tNow("app.discard"),
    });
    if (!restore) {
      await markClean();
      return false;
    }
    if (info.chapter && info.originalPath) {
      // A chapter of a project: open the project there, then put the changes back.
      await openProject(info.originalPath, info.chapter, true);
      const p = projectRef.current;
      if (!p || p.chapter !== info.chapter) return true; // the project couldn't open; its error shows
      const r = await api.restoreRecovery();
      editor.storage.texloom.project = null; // see openChapter
      showDocument(editor, r.doc);
      setProject({ ...p });
      setBibliography(r.bibliography ?? "");
      setMetadata(r.metadata);
      setDirty(true);
      dirtyRef.current = true;
      compile();
      return true;
    }
    const r = await api.restoreRecovery();
    showDocument(editor, r.doc);
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
      const question = doc && tpl ? "app.closeBothUnsaved" : doc ? "app.closeDocUnsaved" : "app.closeTemplateUnsaved";
      const close = await ask(tNow(question), {
        title: tNow("app.unsavedChanges"),
        kind: "warning",
        okLabel: tNow("app.closeWithoutSaving"),
        cancelLabel: tNow("common.cancel"),
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
  const templateNowRef = useRef(template);
  templateNowRef.current = template;

  // How formulas are drawn on the page: the Arabic way when the template says so.
  const mathLang = template?.language;
  useEffect(() => {
    const arabicDocument = mathLang?.main === "arabic";
    setMathSettings({ arabicDocument, arabic: arabicDocument && mathLang?.math === "arabic", indic: arabicDocument && mathLang?.digits === "arabicIndic" });
  }, [mathLang?.main, mathLang?.math, mathLang?.digits]);

  // Node views (references, citations) read the template from the editor.
  const activeTemplate = templateInfo?.template ?? null;
  useEffect(() => {
    if (editor && activeTemplate) {
      // Paragraphs switch to the other direction relative to the document's.
      editor.storage.direction.rtl = docLanguage(activeTemplate).rtl;
      setTexloomContext(editor, { template: activeTemplate });
    }
  }, [editor, activeTemplate]);

  // A chapter of a project: numbering continues from the chapters before it,
  // references to other chapters show their numbers, citations count in book order.
  const openTargetRef = useRef<(chapter: string, id: string) => void>(() => {});
  const projectIndexData = project?.index;
  const projectChapter = project?.chapter;
  useEffect(() => {
    if (!editor || !activeTemplate) return;
    if (!projectIndexData || !projectChapter) {
      if (editor.storage.texloom.project) setTexloomContext(editor, { project: null });
      return;
    }
    const chapters = projectIndexData.chapters;
    const at = chapters.findIndex((c) => c.file === projectChapter);
    const { start, others } = bookNumbering(chapters, projectChapter, activeTemplate);
    setTexloomContext(editor, {
      project: {
        start,
        others,
        citationsBefore: chapters.slice(0, Math.max(at, 0)).flatMap((c) => c.citations),
        citationsAfter: at < 0 ? [] : chapters.slice(at + 1).flatMap((c) => c.citations),
        openTarget: (chapter, id) => openTargetRef.current(chapter, id),
      },
    });
  }, [editor, activeTemplate, projectIndexData, projectChapter]);
  // Fonts from the font library the page uses (the designer's draft included).
  useEffect(() => {
    for (const t of [template, draft]) {
      if (t) libraryFontIds(templateFonts(t)).forEach((id) => void ensureLibraryFont(id));
    }
  }, [template, draft]);
  const lang = docLanguage(template);

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
    const target = path ?? (await open({ filters: templateFilter(), multiple: false, directory: false }));
    return target ? loadTemplateWith(() => api.openTemplate(target)) : false;
  };

  /** Imports a template, template package or publisher kit into My templates. */
  const importTemplateFile = async (): Promise<boolean> => {
    const source = await open({
      multiple: false,
      directory: false,
      filters: [{ name: tr("app.filterTemplateImport"), extensions: ["json", "zip"] }],
    });
    if (!source) return false;
    const ok = await loadTemplateWith(() => api.importTemplate(source));
    refreshUserTemplates();
    return ok;
  };

  const exportTemplateFile = async () => {
    if (!template) return;
    const target = await save({ filters: [{ name: tr("app.filterTemplatePackage"), extensions: ["zip"] }], defaultPath: `${template.meta.id}.zip` });
    if (target) await api.exportTemplate(target).catch(report);
  };

  const removeUserTemplate = async (t: api.UserTemplate) => {
    if (!(await ask(tr("app.removeTemplatePrompt", { name: t.name }), { title: tr("app.removeTemplateTitle"), kind: "warning" }))) return;
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
      if (
        pdf &&
        (await ask(`${tr("app.themeNeedsInstalledTex")}\n\n${tr("app.switchToPdflatex")}`, {
          title: tr("app.texEngineTitle"),
          kind: "info",
          okLabel: tr("app.usePdflatex"),
          cancelLabel: tr("app.keepEngine", { engine: "Tectonic" }),
        }))
      )
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
        const why = tr(engine === "tectonic" ? "app.classInstalledWhy" : "app.classPdflatexOnly");
        const sw = await ask(`${why}\n\n${tr("app.switchToPdflatex")}`, {
          title: tr("app.texEngineTitle"),
          kind: "info",
          okLabel: tr("app.usePdflatex"),
          cancelLabel: tr("app.keepEngine", { engine: engine === "tectonic" ? "Tectonic" : engine === "xelatex" ? "XeLaTeX" : "LuaLaTeX" }),
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
      const defaultName = draftNeedsSaveAs || !userTemplates.some((u) => u.name === name) ? name : tNow("app.copyName", { name });
      const choice = await new Promise<SaveAsChoice | null>((resolve) => setSaveAsRequest({ defaultName, title, resolve }));
      setSaveAsRequest(null);
      if (!choice) return null;
      if (choice.kind === "file") {
        const target = await save({ filters: templateFilter(), defaultPath: `${t.meta.id}.json` });
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
      const info = await saveDraftAs(t, tr("app.saveTemplateFirst"));
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

  /** Creates a blank template in My templates and opens it in the designer. */
  const createBlankTemplate = async (name: string, spec: api.BlankSpec): Promise<boolean> => {
    if (draftDirty && !(await ask(tr("app.discardTemplateChanges"), { title: tr("app.newTemplateTitle"), kind: "warning" }))) return false;
    try {
      applyTemplate(await api.createBlankTemplate(name, spec));
      refreshUserTemplates();
      setMode("template");
      return true;
    } catch (e) {
      report(e);
      return false;
    }
  };

  const newTemplateCopy = () => {
    if (!draft) return;
    const copy = structuredClone(draft);
    copy.meta.id = `${copy.meta.id}-copy`;
    copy.meta.name = tr("app.copyName", { name: copy.meta.name });
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

  /** After a save: with compiling while typing off, "Compile after saving" updates the preview. */
  const compileAfterSave = useCallback(() => {
    const s = settingsRef.current;
    if (s.compileOnSave && !s.autoCompile) compile();
  }, [compile, settingsRef]);

  const saveAs = useCallback(async () => {
    if (!editor) return;
    // A project's chapters are saved in place (Join makes a single document).
    if (projectRef.current) return saveProjectFiles().then(compileAfterSave, report);
    const target = await save({ filters: docFilter(), defaultPath: path ?? inFolder(settingsRef.current.defaultFolder, "report.loom") });
    if (!target) return;
    await api.saveDocument(target, docState()).then(() => {
      setPath(target);
      addRecent(target);
      markClean();
      compileAfterSave();
    }, report);
  }, [editor, path, markClean, addRecent, settingsRef, compileAfterSave]);

  /** Saves the document and its images as one .loom file, to email or upload. */
  const saveAsSingleFile = async () => {
    if (!editor) return;
    const target = await save({
      title: tr("app.saveSingleFileTitle"),
      filters: docFilter(),
      defaultPath: path ?? inFolder(settingsRef.current.defaultFolder, "report.loom"),
    });
    if (!target) return;
    await api.saveDocument(target, docState(), true).then(() => {
      setPath(target);
      addRecent(target);
      markClean();
      compileAfterSave();
    }, report);
  };

  const saveDoc = useCallback(async () => {
    if (!editor) return;
    if (projectRef.current) return saveProjectFiles().then(compileAfterSave, report);
    if (!path) return saveAs();
    await api.saveDocument(path, docState()).then(() => {
      markClean();
      compileAfterSave();
    }, report);
  }, [editor, path, saveAs, markClean, compileAfterSave]);

  // ---------------------------------------------------------------- projects

  /** Saves the open chapter and the project file (metadata, bibliography, chapter list). */
  const saveProjectFiles = async () => {
    const p = projectRef.current;
    const ed = editorRef.current;
    if (!p || !ed) return;
    await api.saveChapter(p.chapter, ed.getJSON());
    const index = await api.saveProject(metadataRef.current, bibRef.current, p.entries);
    const cur = projectRef.current;
    if (cur) setProject({ ...cur, index });
    await markClean();
  };

  /** Leaves project mode (a single document is opened or created instead). */
  const leaveProject = () => {
    if (!projectRef.current) return;
    setProject(null);
    api.closeProject().catch(() => {});
  };

  /** Puts the cursor on the target with `id` in the editor's document and marks it. */
  const goToTarget = (id: string) => {
    if (!editor) return;
    let found = -1;
    let atom = false;
    editor.state.doc.descendants((node, pos) => {
      if (found >= 0) return false;
      if (node.attrs?.id === id) {
        found = pos;
        atom = node.isAtom;
      }
      return true;
    });
    if (found < 0) return;
    // An equation is one unit: select it; elsewhere put the cursor inside.
    const chain = editor.chain().focus();
    (atom ? chain.setNodeSelection(found) : chain.setTextSelection(Math.min(found + 1, editor.state.doc.content.size))).scrollIntoView().run();
    flashBlock(editor, found);
  };

  /** Opens chapter `file` in the editor (saving the open one first), then optionally
   *  goes to a target in it or to a top-level block. */
  const openChapter = async (file: string, then?: { target?: string; block?: number }) => {
    const p = projectRef.current;
    if (!p || !editor) return;
    try {
      if (file !== p.chapter) {
        if (dirtyRef.current) await saveProjectFiles();
        const doc = await api.readChapter(file);
        // The context still describes the previous chapter, whose "other chapters"
        // include this one: its ids would look like duplicates and be renamed. The
        // effect sets the new chapter's context once the state below is applied.
        editor.storage.texloom.project = null;
        loadDocument(editor, doc);
        setProject({ ...(projectRef.current ?? p), chapter: file });
        if (!then) focusStart(editor, editorScroll.current);
        compile();
      }
      if (then?.target) window.setTimeout(() => goToTarget(then.target!), 0);
      if (then?.block != null) window.setTimeout(() => jumpToBlock(then.block!), 0);
    } catch (e) {
      report(e);
    }
  };
  openTargetRef.current = (chapter, id) => void openChapter(chapter, { target: id });

  /** Opens a project, at `chapter` (relative to the project folder) or its first one.
   *  `keepRecovery`: restoring a crash's changes into it (the copy must stay). */
  const openProject = async (target: string, chapter?: string, keepRecovery = false, recent = true) => {
    if (!editor) return;
    try {
      const opened = await api.openProject(target);
      const files = opened.chapters.flatMap((e) => ("file" in e ? [e.file] : []));
      const first = chapter && files.includes(chapter) ? chapter : files.find((f) => !opened.issues.some((i) => i.path === `chapters.${f}`)) ?? files[0];
      const doc = first ? await api.readChapter(first) : EMPTY_DOC;
      // Not the example project: its folder is temporary.
      if (recent) addRecent(target);
      editor.storage.texloom.project = null; // see openChapter
      showDocument(editor, doc);
      focusStart(editor, editorScroll.current);
      setBibliography(opened.bibliography ?? "");
      metadataRef.current = opened.metadata;
      setPath(target);
      setProject({ path: target, entries: opened.chapters, index: opened.index, chapter: first ?? "", issues: opened.issues });
      if (keepRecovery) {
        setDirty(false);
        dirtyRef.current = false;
      } else markClean();
      setPanel("chapters");
      if (opened.template) applyTemplate(opened.template);
      else {
        setMetadata(opened.metadata);
        compile();
      }
    } catch (e) {
      if (settings.recentFiles.includes(target)) removeRecent(target);
      report(e);
    }
  };

  /** A changed chapter list (reordered, parts, appendices, removed): saved at once. */
  const changeEntries = async (entries: api.ProjectEntry[]) => {
    const p = projectRef.current;
    if (!p) return;
    try {
      const index = await api.saveProject(metadataRef.current, bibRef.current, entries);
      const next = { ...(projectRef.current ?? p), entries, index };
      setProject(next);
      const files = entries.flatMap((e) => ("file" in e ? [e.file] : []));
      // The open chapter was removed: open the first one (without saving it back).
      if (!files.includes(next.chapter) && files[0]) {
        markClean();
        await openChapter(files[0]);
      } else compile();
    } catch (e) {
      report(e);
    }
  };

  /** Adds a new chapter (or appendix) at the end and opens it. */
  const addChapter = async (appendix: boolean) => {
    const p = projectRef.current;
    if (!p) return;
    try {
      const file = await api.newChapter(tr(appendix ? "project.newAppendixTitle" : "project.newChapterTitle"), appendix);
      const boundary = p.entries.findIndex((e) => "file" in e && e.appendix);
      const entry: api.ProjectEntry = appendix ? { file, appendix: true } : { file };
      const entries = appendix || boundary < 0 ? [...p.entries, entry] : [...p.entries.slice(0, boundary), entry, ...p.entries.slice(boundary)];
      const index = await api.saveProject(metadataRef.current, bibRef.current, entries);
      setProject({ ...(projectRef.current ?? p), entries, index });
      await openChapter(file);
    } catch (e) {
      report(e);
    }
  };

  /** What compiles and exports of the project send: the open chapter, live. */
  const projectState = (): api.ProjectState => ({ file: projectRef.current?.chapter ?? null, ...docState() });

  /** Turns the open document into a project in a folder the user picks, and opens it. */
  const splitIntoProject = async () => {
    if (!editor) return;
    const folder = await open({ directory: true, multiple: false, title: tr("project.splitFolderTitle"), defaultPath: settings.defaultFolder || undefined });
    if (!folder) return;
    const name = path ? fileNameOf(path).replace(/\.loom$/, "") : folder.split(/[\\/]/).filter(Boolean).pop() ?? "book";
    try {
      const projectPath = await api.splitIntoProject(folder, name, docState());
      // Its content is in the project now; the document itself is left as it was.
      await markClean();
      await openProject(projectPath);
    } catch (e) {
      report(e);
    }
  };

  /** A new project with the current template: one chapter, in a folder the user
   *  picks (the folder's name names the project). */
  const newProject = async () => {
    if (!editor) return;
    if (dirty && !(await ask(tr("app.discardAndNew"), { title: tr("app.newDocumentTitle"), kind: "warning" }))) return;
    // The template window chooses the project's template; then the folder.
    setStartWindow(false);
    setProjectPick(true);
    setShowGallery(true);
  };

  /** The project's template was picked in the template window: asks for the folder
   *  and creates the project (its first chapter) with that template. */
  const createProject = async (choose: () => Promise<boolean>, recentKey?: string) => {
    setShowGallery(false);
    setProjectPick(false);
    if (!(await choose())) return;
    if (recentKey) updateSettings((s) => ({ recentTemplates: [recentKey, ...s.recentTemplates.filter((k) => k !== recentKey)].slice(0, 5) }));
    const folder = await open({ directory: true, multiple: false, title: tr("project.newProjectFolderTitle"), defaultPath: settings.defaultFolder || undefined });
    if (!folder) {
      // No folder: an empty document with the chosen template, as from the gallery.
      await newDoc(true);
      return;
    }
    const name = folder.split(/[\\/]/).filter(Boolean).pop() ?? "book";
    // The (new) template's language decides the first chapter's title. The folder
    // dialog let React render, so the ref holds the chosen template.
    const first = translate(docLanguage(templateNowRef.current).main === "arabic" ? "ar" : "en", "project.firstChapterTitle");
    const doc: JSONContent = { type: "doc", content: [{ type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: first }] }, { type: "paragraph" }] };
    try {
      const projectPath = await api.splitIntoProject(folder, name, { doc, metadata: {}, bibliography: "" });
      await markClean();
      await openProject(projectPath);
    } catch (e) {
      report(e);
    }
  };

  /** Writes the whole project as one document and opens it. */
  const joinProject = async () => {
    const p = projectRef.current;
    if (!p) return;
    const target = await save({ filters: docFilter(), defaultPath: p.path.replace(/\.loomproj$/, ".loom") });
    if (!target) return;
    try {
      await api.joinProject(target, projectState());
      await openDoc(target);
    } catch (e) {
      report(e);
    }
  };

  /** Opens a document from the dialog, or `target` (a recent file); projects too. */
  const openDoc = async (target?: string) => {
    if (!editor) return;
    if (dirty && !(await ask(tr("app.discardAndOpen"), { title: tr("app.openDocumentTitle"), kind: "warning" }))) return;
    if (!target) {
      const picked = await open({ filters: openFilter(), multiple: false, directory: false, defaultPath: settings.defaultFolder || undefined });
      if (!picked) return;
      target = picked;
    }
    if (target.toLowerCase().endsWith(".loomproj")) return openProject(target);
    try {
      const opened = await api.openDocument(target);
      if (opened.project) {
        // A chapter of a project: open the project at that chapter.
        const folder = opened.project.replace(/[\\/][^\\/]*$/, "");
        return openProject(opened.project, relativeTo(folder, target) ?? undefined);
      }
      leaveProject();
      addRecent(target);
      showDocument(editor, opened.doc);
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
    const name = (path ? fileNameOf(path).replace(/\.loom(proj)?$/, "") : "report") + ".pdf";
    const target = await save({ filters: [{ name: "PDF", extensions: ["pdf"] }], defaultPath: path ? name : inFolder(settings.defaultFolder, name) });
    if (target) await (projectRef.current ? api.exportProjectPdf(target, projectState()) : api.exportPdf(target, docState())).catch(report);
  };

  const exportLatex = async () => {
    if (!editor) return;
    const target = await save({ filters: [{ name: "LaTeX", extensions: ["tex"] }], defaultPath: inFolder(settings.defaultFolder, "report.tex") });
    if (target) await (projectRef.current ? api.exportProjectTex(target, projectState()) : api.exportTex(target, docState())).catch(report);
  };

  const exportWord = async () => {
    if (!editor) return;
    const name = (path ? fileNameOf(path).replace(/\.loom(proj)?$/, "") : "report") + ".docx";
    const target = await save({ filters: [{ name: "Word", extensions: ["docx"] }], defaultPath: path ? name : inFolder(settings.defaultFolder, name) });
    if (!target) return;
    const state = docState();
    const p = projectRef.current;
    if (p) {
      // Formulas of every chapter: the open one live, the others as saved.
      const files = p.entries.flatMap((e) => ("file" in e ? [e.file] : []));
      try {
        const docs = await Promise.all(files.map((f) => (f === p.chapter ? Promise.resolve(state.doc) : api.readChapter(f))));
        const mathml = Object.assign({}, ...docs.map((d) => formulasAsMathMl(d)));
        await api.exportProjectDocx(target, projectState(), mathml, await arabicFormulaPictures(docs));
      } catch (e) {
        report(e);
      }
      return;
    }
    await api.exportDocx(target, state, formulasAsMathMl(state.doc), await arabicFormulaPictures([state.doc])).catch(report);
  };

  const newDoc = async (skipAsk = false) => {
    if (!editor) return;
    if (!skipAsk && dirty && !(await ask(tr("app.discardAndNew"), { title: tr("app.newDocumentTitle"), kind: "warning" }))) return;
    try {
      const fresh = await api.newDocument();
      leaveProject();
      showDocument(editor, fresh.doc);
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

  /** An example document with its template: in Arabic when the interface is (the
   *  Arabic journal article and slides), else the senior design report and slides. */
  const openExample = async (which: "report" | "presentation" | "project" = "report") => {
    if (!editor) return;
    if (dirty && !(await ask(tr("app.discardAndExample"), { title: tr("app.exampleTitle"), kind: "warning" }))) return;
    setShowGallery(false);
    setStartWindow(false);
    try {
      if (which === "project") {
        // The Arabic thesis (in two parts) or the report, one file per chapter, in a
        // temporary folder.
        const projectPath = await api.sampleProject(uiLang === "ar" ? "arabic-thesis" : undefined);
        await markClean();
        await openProject(projectPath, undefined, false, false);
        return;
      }
      const deck = which === "presentation";
      const arabic = uiLang === "ar";
      const [templateId, sampleName] = arabic
        ? deck ? (["arabic-presentation", "arabic-presentation"] as const) : (["arabic-article", "arabic-article"] as const)
        : deck ? (["beamer-presentation", "presentation"] as const) : (["senior-design", undefined] as const);
      const [t, sample] = await Promise.all([api.switchToBuiltinTemplate(templateId), api.sampleDocument(sampleName)]);
      leaveProject();
      showDocument(editor, sample.doc);
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
    if (dirty && !(await ask(tr("app.discardAndNew"), { title: tr("app.newDocumentTitle"), kind: "warning" }))) return;
    setShowGallery(false);
    if (!(await choose())) return;
    // "Recently used" in the template window: the last five, newest first.
    if (recentKey) updateSettings((s) => ({ recentTemplates: [recentKey, ...s.recentTemplates.filter((k) => k !== recentKey)].slice(0, 5) }));
    await newDoc(true);
  };

  /** SyncTeX, PDF → editor: puts the cursor in the block behind a double-clicked spot. */
  const jumpFromPdf = async (page: number, x: number, y: number) => {
    const p = projectRef.current;
    if (p) {
      const spot = await api.projectPdfToBlock(page, x, y).catch(() => null);
      if (spot) await openChapter(spot.file, { block: spot.block });
      return;
    }
    const index = await api.pdfToBlock(page, x, y).catch(() => null);
    if (index != null) jumpToBlock(index);
  };

  /** From the error list: a block of the open chapter, or of another chapter. */
  const jumpToError = (block: number, chapter?: string) => {
    const p = projectRef.current;
    if (p && chapter && chapter !== p.chapter) void openChapter(chapter, { block });
    else jumpToBlock(block);
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
  const describeBlock = (index: number, chapter?: string): string => {
    const p = projectRef.current;
    if (p && chapter && chapter !== p.chapter) {
      // Another chapter isn't loaded: name the chapter.
      return p.index.chapters.find((c) => c.file === chapter)?.title || chapter;
    }
    const node = editor?.state.doc.maybeChild(index);
    if (!node) return "";
    const kind: Record<string, MessageKey> = {
      heading: "app.blockHeading",
      paragraph: "app.blockParagraph",
      figure: "app.blockFigure",
      tableFigure: "app.blockTable",
      table: "app.blockTable",
      equation: "app.blockEquation",
      rawLatex: "app.blockRawLatex",
      bulletList: "app.blockList",
      orderedList: "app.blockList",
      blockquote: "app.blockQuote",
      codeBlock: "app.blockCode",
      slide: "app.blockSlide",
    };
    const text = node.textContent.trim().replace(/\s+/g, " ");
    const name = tr(kind[node.type.name] ?? "app.blockOther");
    return text ? tr("app.blockWithText", { kind: name, text: text.length > 40 ? text.slice(0, 40) + "…" : text }) : name;
  };

  /** SyncTeX, editor → PDF: scrolls the preview to the block with the cursor. */
  const showInPdf = async () => {
    if (!editor) return;
    const block = editor.state.selection.$from.index(0);
    const p = projectRef.current;
    const spot = await (p ? api.projectBlockToPdf(p.chapter, block) : api.blockToPdf(block)).catch(() => null);
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

  useEffect(() => {
    if (editor) editor.storage.arabicPunctuation.enabled = settings.arabicPunctuation;
  }, [editor, settings.arabicPunctuation]);

  // Spell checking follows the setting.
  useEffect(() => {
    editor?.view.dom.setAttribute("spellcheck", String(settings.spellcheck));
  }, [editor, settings.spellcheck]);

  // Words in the document body, for the status bar: counted after a pause in typing,
  // not on every keystroke (counting a long book's text takes tens of milliseconds).
  const [wordCount, setWordCount] = useState(0);
  useEffect(() => {
    if (!editor) return;
    let timer = 0;
    const count = () => setWordCount(countWords(editor.state.doc.textBetween(0, editor.state.doc.content.size, " ", " ")));
    // Any change of the document, typed or loaded (opening a file replaces it).
    const later = ({ transaction }: { transaction: { docChanged: boolean } }) => {
      if (!transaction.docChanged) return;
      window.clearTimeout(timer);
      timer = window.setTimeout(count, 400);
    };
    count();
    editor.on("transaction", later);
    return () => {
      window.clearTimeout(timer);
      editor.off("transaction", later);
    };
  }, [editor]);

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

  const fileName = path ? fileNameOf(path) : tr("app.untitled");
  const missing = issues.filter((i) => i.path.startsWith("metadata.") && i.message.includes("required")).length;
  const headingNames =
    template?.structure.topLevel === "section"
      ? [tr("app.styleSection"), tr("app.styleSubsection"), tr("app.styleSubsubsection"), tr("app.styleParagraph")]
      : [tr("app.styleChapter"), tr("app.styleSection"), tr("app.styleSubsection"), tr("app.styleSubsubsection")];
  const draftErrors = draftIssues.filter((i) => i.severity === "error").length;
  // A chapter of a project: the page's own numbers (CSS counters) start where the
  // chapters before it end, and an appendix chapter numbers with letters.
  const chapterStart = project && template ? bookNumbering(project.index.chapters, project.chapter, template).start : null;
  const chapterIsAppendix = !!project?.index.chapters.find((c) => c.file === project.chapter)?.appendix;
  const counterCss = chapterStart
    ? `.editor-pane .page .tiptap{counter-reset:tl-h1 ${chapterStart.levels[0]} tl-h2 0 tl-h3 0 tl-h4 0 tl-figure ${chapterStart.figure} tl-table ${chapterStart.table} tl-equation ${chapterStart.equation}}`
    : "";
  const openChapterTitle = project ? project.index.chapters.find((c) => c.file === project.chapter)?.title || tr("project.untitledChapter") : "";
  // Words in the whole book: the other chapters (as indexed) plus this one, live.
  const bookWords = project ? project.index.chapters.reduce((n, c) => n + (c.file === project.chapter ? 0 : c.words), 0) + wordCount : 0;
  const panelTitle: Record<SidePanel, string> = {
    chapters: tr("project.panel"),
    outline: tr("app.panelOutline"),
    info: tr("app.panelInfo"),
    refs: tr("app.panelRefs"),
    checks: tr("app.panelChecks"),
  };

  return (
    <LanguageContext.Provider value={uiLang}>
    <div className="app">
      <header className="titlebar">
        <div className="titlebar-side">
          <Menu label={<><MenuIcon size={16} /><span>{tr("app.menuFile")}</span></>} className="tb-button">
            {(close) => {
              const act = (f: () => unknown) => () => {
                close();
                f();
              };
              return (
                <>
                  <MenuItem icon={<FilePlus2 />} label={tr("app.menuNew")} shortcut="Ctrl+N" onClick={act(() => newDoc())} title={tr("app.menuNewTitle")} />
                  <MenuItem icon={<LayoutTemplate />} label={tr("app.menuNewFromTemplate")} shortcut="Ctrl+Shift+N" onClick={act(() => openGallery())} title={tr("app.menuNewFromTemplateTitle")} />
                  <MenuItem icon={<Files />} label={tr("project.newProject")} onClick={act(newProject)} title={tr("project.newProjectTitle")} />
                  <MenuItem icon={<FolderOpen />} label={tr("app.menuOpen")} shortcut="Ctrl+O" onClick={act(() => openDoc())} />
                  <MenuItem icon={<Sparkles />} label={tr("app.menuOpenExampleReport")} onClick={act(() => openExample())} />
                  <MenuItem icon={<Sparkles />} label={tr("app.menuOpenExamplePresentation")} onClick={act(() => openExample("presentation"))} />
                  <MenuItem icon={<Sparkles />} label={tr("project.menuOpenExample")} onClick={act(() => openExample("project"))} title={tr("project.openExampleTitle")} />
                  {settings.recentFiles.length > 0 && (
                    <>
                      <MenuHeading>{tr("app.menuRecent")}</MenuHeading>
                      {settings.recentFiles.map((f) => (
                        <MenuItem key={f} icon={<History />} label={fileNameOf(f)} title={f} current={f === path} onClick={act(() => openDoc(f))} />
                      ))}
                    </>
                  )}
                  <MenuSeparator />
                  <MenuItem icon={<Save />} label={tr("common.save")} shortcut="Ctrl+S" onClick={act(saveDoc)} />
                  {!project && (
                    <>
                      <MenuItem label={tr("app.menuSaveAs")} shortcut="Ctrl+Shift+S" onClick={act(saveAs)} />
                      <MenuItem
                        label={tr("app.menuSaveSingleFile")}
                        onClick={act(saveAsSingleFile)}
                        title={tr("app.menuSaveSingleFileTitle")}
                      />
                    </>
                  )}
                  {!project && !isDeck && (
                    <MenuItem icon={<Split />} label={tr("project.menuSplit")} onClick={act(splitIntoProject)} title={tr("project.menuSplitTitle")} />
                  )}
                  {project && <MenuItem icon={<Merge />} label={tr("project.menuJoin")} onClick={act(joinProject)} title={tr("project.menuJoinTitle")} />}
                  <MenuSeparator />
                  <MenuItem icon={<FileDown />} label={tr("app.menuExportPdf")} shortcut="Ctrl+Shift+E" onClick={act(exportPdfFile)} />
                  <MenuItem
                    icon={<FileCode2 />}
                    label={tr("app.menuExportLatex")}
                    onClick={act(exportLatex)}
                    title={tr("app.menuExportLatexTitle")}
                  />
                  <MenuItem
                    icon={<FileText />}
                    label={tr("app.menuExportWord")}
                    onClick={act(exportWord)}
                    disabled={isDeck}
                    title={isDeck ? tr("app.menuExportWordDeck") : tr("app.menuExportWordTitle")}
                  />
                  <MenuSeparator />
                  <MenuItem icon={<SettingsIcon />} label={tr("app.settings")} shortcut="Ctrl+," onClick={act(() => setShowSettings(true))} />
                </>
              );
            }}
          </Menu>
          <div className="doc-title" title={path ?? tr("app.notSavedYet")}>
            {project ? <Files size={15} aria-hidden="true" /> : <FileText size={15} aria-hidden="true" />}
            <span className="doc-name">{fileName}</span>
            {project && <span className="doc-chapter">— {openChapterTitle}</span>}
            {dirty && <span className="dirty-dot" title={tr("app.unsavedChanges")} />}
          </div>
        </div>
        <div className="mode-switch" role="tablist" aria-label={tr("app.mode")}>
          <button role="tab" aria-selected={mode === "document"} className={mode === "document" ? "active" : ""} onClick={() => setMode("document")}>
            <PenLine size={15} aria-hidden="true" /> {tr("app.modeWrite")}
          </button>
          <button role="tab" aria-selected={mode === "template"} className={mode === "template" ? "active" : ""} onClick={() => setMode("template")}>
            <LayoutTemplate size={15} aria-hidden="true" /> {tr("app.modeDesign")}
            {draftDirty && <span className="dirty-dot" title={tr("app.unsavedTemplateChanges")} />}
            {draftErrors > 0 && <span className="badge">{draftErrors}</span>}
          </button>
        </div>
        <div className="titlebar-side right">
          <Menu
            align="right"
            className="tb-button template-button"
            title={tr("app.templateTooltip", {
              name: template?.meta.name ?? "",
              where: templateInfo?.path ?? (currentTexClass ? tr("app.templateFromTex", { cls: currentTexClass }) : tr("app.templateBuiltIn")),
            })}
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
                    (currentTexClass ? tr("app.templateFromTex", { cls: currentTexClass }) : tr("app.templateBuiltIn"))}
                </span>
                {templateInfo?.path && <span className="muted">{tr("app.templateReloads")}</span>}
                <span className="muted">{tr("app.templateRestyles")}</span>
              </div>
              <MenuItem
                icon={<FilePlus2 />}
                label={tr("app.newDocFromTemplate")}
                onClick={() => {
                  close();
                  openGallery();
                }}
              />
              <MenuItem
                icon={<FileInput />}
                label={tr("app.importTemplate")}
                title={tr("app.importTemplateTitle")}
                onClick={() => {
                  close();
                  importTemplateFile();
                }}
              />
              <MenuItem
                icon={<FileOutput />}
                label={tr("app.exportTemplate")}
                title={tr("app.exportTemplateTitle")}
                onClick={() => {
                  close();
                  exportTemplateFile();
                }}
              />
              <MenuItem
                icon={<FilePlus2 />}
                label={tr("app.newBlankTemplate")}
                title={tr("app.newBlankTemplateTitle")}
                onClick={() => {
                  close();
                  setShowNewTemplate(true);
                }}
              />
              <MenuItem
                icon={<FolderOpen />}
                label={tr("app.openTemplateFile")}
                title={tr("app.openTemplateFileTitle")}
                onClick={() => {
                  close();
                  openTemplateFile();
                }}
              />
              {userTemplates.length > 0 && (
                <div>
                  <MenuHeading>{tr("app.myTemplates")}</MenuHeading>
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
                { title: tr("app.groupReports"), items: builtins.filter((b) => !b.class && b.kind !== "presentation") },
                { title: tr("app.groupPresentations"), items: builtins.filter((b) => b.kind === "presentation") },
                { title: tr("app.groupJournals"), items: builtins.filter((b) => b.class) },
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
                  <MenuHeading>{tr("app.fromTexInstallation")}</MenuHeading>
                  {currentTexClass && (
                    <MenuItem current icon={<CircleCheck />} label={template?.meta.name ?? currentTexClass} shortcut={currentTexClass} onClick={close} />
                  )}
                  <MenuItem
                    icon={<Library />}
                    label={tr("app.chooseFromClasses", { n: texTemplates.length })}
                    title={tr("app.chooseFromClassesTitle")}
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
          <button className="tb-button" onClick={exportPdfFile} title={tr("app.exportPdfTitle")}>
            <FileDown size={16} aria-hidden="true" /> {tr("app.exportPdf")}
          </button>
          <button className="icon-button" onClick={() => setShowSettings(true)} title={tr("app.settingsTitle")} aria-label={tr("app.settings")}>
            <SettingsIcon size={17} />
          </button>
        </div>
      </header>

      {editor && mode === "document" && <FormatBar editor={editor} headingNames={headingNames} onError={report} onPicker={setPicker} deck={isDeck} onThemes={() => setShowThemePicker(true)} />}

      <main className={`workspace${mode === "template" ? " template-mode" : ""}`}>
        {mode === "document" && (
          <nav className="rail" aria-label={tr("app.panels")}>
            {project && <RailButton icon={<Files />} label={panelTitle.chapters} active={panel === "chapters"} onClick={() => togglePanel("chapters")} />}
            <RailButton icon={<ListTree />} label={panelTitle.outline} active={panel === "outline"} onClick={() => togglePanel("outline")} />
            <RailButton icon={<IdCard />} label={panelTitle.info} badge={missing || undefined} active={panel === "info"} onClick={() => togglePanel("info")} />
            <RailButton
              icon={<Library />}
              label={panelTitle.refs}
              badge={bibInfo.error ? "!" : undefined}
              active={panel === "refs"}
              onClick={() => togglePanel("refs")}
            />
            <RailButton
              icon={<TriangleAlert />}
              label={panelTitle.checks}
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
              <button className="icon-button small" onClick={() => setPanel(null)} aria-label={tr("app.closePanel")} title={tr("common.close")}>
                <X size={15} />
              </button>
            </div>
            <div className="side-body">
              {panel === "chapters" && project && (
                <ChaptersPanel
                  entries={project.entries}
                  index={project.index}
                  current={project.chapter}
                  onOpen={(file) => void openChapter(file)}
                  onEntries={changeEntries}
                  onNewChapter={addChapter}
                />
              )}
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
              onNewBlank={() => setShowNewTemplate(true)}
              onImportKit={importKit}
              onBrowseThemes={() => setShowThemePicker(true)}
            />
          </section>
        )}
        {/* Kept mounted (just hidden) in template mode so the editor keeps its state. */}
        <section className="editor-pane" hidden={mode !== "document"}>
          {template && <style>{templateCss(template, ".editor-pane .page", deckInfo)}</style>}
          {counterCss && <style>{counterCss}</style>}
          {/* The editor's own header, like the preview's: the page's zoom in steps of
              10; the value goes back to 100%. */}
          <div className="pane-head">
            <span className="pane-title">{tr("app.editor")}</span>
            <span className="spacer" />
            <span className="zoom" role="group" aria-label={tr("app.pageZoom")}>
              <button className="icon-button small" onMouseDown={(e) => e.preventDefault()} onClick={() => updateSettings({ pageZoom: Math.max(50, Math.ceil(shownZoom / 10) * 10 - 10) })} disabled={shownZoom <= 50} aria-label={tr("app.zoomOut")} title={tr("app.zoomOut")}>
                <ZoomOut size={15} />
              </button>
              <button className="zoom-value" onMouseDown={(e) => e.preventDefault()} onClick={() => updateSettings({ pageZoom: 100 })} title={tr("app.pageZoomReset")}>
                {shownZoom}%
              </button>
              <button className="icon-button small" onMouseDown={(e) => e.preventDefault()} onClick={() => updateSettings({ pageZoom: Math.min(200, Math.floor(shownZoom / 10) * 10 + 10) })} disabled={shownZoom >= 200} aria-label={tr("app.zoomIn")} title={tr("app.zoomIn")}>
                <ZoomIn size={15} />
              </button>
            </span>
          </div>
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
            <div
              className={`page${isDeck ? " deck" : ""}${chapterIsAppendix ? " appendix-chapter" : ""}`}
              dir={lang.rtl ? "rtl" : "ltr"}
              lang={lang.tag}
              style={logoUrl ? { ["--slide-logo" as string]: `url("${logoUrl}")` } : undefined}
            >
              <EditorContent editor={editor} />
            </div>
          </div>
        </section>
        <section className="preview-pane">
          <div className="pane-head">
            <span className="pane-title">{showLatex ? tr("app.latexSource") : tr("app.preview")}</span>
            {mode === "template" && !showLatex && (
              <span className="segmented" role="group" aria-label={tr("app.previewKind")}>
                <button className={designPreview === "pdf" ? "active" : ""} aria-pressed={designPreview === "pdf"} onClick={() => setDesignPreview("pdf")} title={tr("app.previewPdfTitle")}>
                  PDF
                </button>
                <button className={designPreview === "page" ? "active" : ""} aria-pressed={designPreview === "page"} onClick={() => setDesignPreview("page")} title={tr("app.previewPageTitle")}>
                  {tr("app.previewPage")}
                </button>
              </span>
            )}
            <span className="spacer" />
            {project && mode === "document" && !showLatex && (
              <span className="segmented" role="group" aria-label={tr("project.scopeTitle")} title={tr("project.scopeTitle")}>
                {[false, true].map((whole) => (
                  <button
                    key={String(whole)}
                    className={wholeBook === whole ? "active" : ""}
                    aria-pressed={wholeBook === whole}
                    onClick={() => {
                      wholeBookRef.current = whole;
                      setWholeBookState(whole);
                      compile();
                    }}
                  >
                    {tr(whole ? "project.scopeBook" : "project.scopeChapter")}
                  </button>
                ))}
              </span>
            )}
            {!settings.autoCompile && (
              <button className="tb-button" onClick={compile} title={tr("app.compileTitle")}>
                <Play size={14} aria-hidden="true" /> {tr("app.compile")}
              </button>
            )}
            {!showLatex && (
              <span className="zoom" role="group" aria-label={tr("app.previewZoom")}>
                <button className="icon-button small" onClick={() => setPreviewZoom((z) => Math.max(50, z - 10))} aria-label={tr("app.zoomOut")} title={tr("app.zoomOut")}>
                  <ZoomOut size={15} />
                </button>
                <button className="zoom-value" onClick={() => setPreviewZoom(100)} title={tr("app.fitWidth")}>
                  {previewZoom}%
                </button>
                <button className="icon-button small" onClick={() => setPreviewZoom((z) => Math.min(200, z + 10))} aria-label={tr("app.zoomIn")} title={tr("app.zoomIn")}>
                  <ZoomIn size={15} />
                </button>
              </span>
            )}
            {mode === "document" && (
              <button
                className="icon-button small"
                onClick={showInPdf}
                disabled={!pdf}
                aria-label={tr("app.showInPdf")}
                title={tr("app.showInPdfTitle")}
              >
                <LocateFixed size={15} />
              </button>
            )}
            {showLatex && (
              <button className="tb-button" onClick={copyLatex} disabled={!latex} title={tr("app.copyLatexTitle")}>
                {latexCopied ? <Check size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />} {tr(latexCopied ? "app.copied" : "app.copyLatex")}
              </button>
            )}
            <button
              className={`icon-button small${showLatex ? " active" : ""}`}
              onClick={() => setShowLatex((v) => !v)}
              aria-label={tr("app.latexSource")}
              title={showLatex ? tr("app.backToPdf") : tr("app.showLatex")}
            >
              <Code size={15} />
            </button>
          </div>
          {templateProblem && <TemplateProblemPanel problem={templateProblem} onDismiss={() => setTemplateProblem(null)} />}
          {status.state === "error" && <ErrorPanel error={status.error} onJump={jumpToError} describe={describeBlock} />}
          {showLatex ? (
            <pre className="latex-source">{latex}</pre>
          ) : mode === "template" && designPreview === "page" && draft ? (
            <PagePreview editor={editor} template={draft} deck={deckInfo} />
          ) : (
            <PdfPreview pdf={pdf} docKey={docKey} zoom={previewZoom} onPages={setPageCount} onLocate={mode === "document" ? jumpFromPdf : undefined} spot={pdfSpot} />
          )}
        </section>
      </main>

      <footer className="statusbar">
        <StatusBadge status={status} ms={compileMs} />
        {pageCount > 0 && <span>{tr(pageCount === 1 ? "app.pageCountOne" : "app.pageCountMany", { n: pageCount })}</span>}
        {mode === "document" &&
          (project ? (
            <span title={tr("project.wordsBookTitle")}>{tr("project.wordsBook", { chapter: wordCount.toLocaleString(), book: bookWords.toLocaleString() })}</span>
          ) : (
            <span>{tr("app.wordCount", { n: wordCount.toLocaleString() })}</span>
          ))}
        {mode === "document" && issues.length > 0 && (
          <button className="status-link warn" onClick={() => setPanel("checks")}>
            <TriangleAlert size={13} aria-hidden="true" /> {tr("app.toCheck", { n: issues.length })}
          </button>
        )}
        <span className="spacer" />
        {mode === "document" && template && (
          <span title={templateInfo?.path ?? tr("app.builtInTemplate")}>
            <LayoutTemplate size={13} aria-hidden="true" /> {template.meta.name}
          </span>
        )}
        {mode === "document" && (
          <Menu
            up
            align="right"
            className="status-link"
            title={tr("app.pageZoom")}
            label={settings.pageZoom ? tr("app.pageZoomValue", { z: settings.pageZoom }) : tr("app.pageZoomFit", { z: Math.round(fitZoom * 100) })}
          >
            {(close) =>
              [0, 75, 90, 100, 110, 125, 150].map((z) => (
                <MenuItem
                  key={z}
                  current={settings.pageZoom === z}
                  icon={settings.pageZoom === z ? <CircleCheck /> : undefined}
                  label={z ? `${z}%` : tr("app.fitWidth")}
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

      {showNewTemplate && <NewTemplateDialog onCreate={createBlankTemplate} onClose={() => setShowNewTemplate(false)} />}
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
          onPick={(id) => (projectPick ? createProject : newFromTemplate)(() => switchToBuiltin(id), `builtin:${id}`)}
          userTemplates={userTemplates}
          currentPath={templateInfo?.path ?? null}
          onPickUser={(u) => (projectPick ? createProject : newFromTemplate)(() => openTemplateFile(u.path), `mine:${u.path}`)}
          onRemoveUser={removeUserTemplate}
          onImport={() => (projectPick ? createProject : newFromTemplate)(importTemplateFile)}
          installed={installedTemplates}
          waiting={installedWaiting}
          installedThemes={installedThemeCount}
          currentClass={currentTexClass}
          onPickInstalled={(c) => (projectPick ? createProject : newFromTemplate)(() => switchToInstalled(c), `tex:${c}`)}
          engine={settings.engine}
          start={startWindow && !projectPick}
          forProject={projectPick}
          view={settings.galleryView}
          category={settings.galleryCategory}
          recentTemplates={settings.recentTemplates}
          onPrefs={(p) => updateSettings(p)}
          recentFiles={settings.recentFiles}
          onOpen={(p) => {
            setShowGallery(false);
            openDoc(p);
          }}
          onNewProject={newProject}
          onExample={() => openExample()}
          onExamplePresentation={() => openExample("presentation")}
          onExampleProject={() => openExample("project")}
          onClose={() => {
            setShowGallery(false);
            setProjectPick(false);
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
    </LanguageContext.Provider>
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
  const t = useT();
  if (issues.length === 0) {
    return (
      <p className="side-empty">
        <CircleCheck size={16} aria-hidden="true" /> {t("app.nothingToCheck")}
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
  const t = useT();
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
      canMergeNext: editor.can().mergeWithNeighbour("next"),
      canMergeBelow: editor.can().mergeWithNeighbour("below"),
      canSplit: editor.can().splitCell(),
      headingNumbered: editor.getAttributes("heading").numbered !== false,
      // The paragraph or heading at the cursor runs the other way than the document.
      otherDir: (editor.getAttributes("paragraph").dir ?? editor.getAttributes("heading").dir ?? null) as string | null,
      // The paragraph's own alignment (null: the template's).
      align: (editor.isActive("paragraph") ? editor.getAttributes("paragraph").align ?? null : null) as string | null,
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
    <div className="toolbar" role="toolbar" aria-label={t("app.formatting")}>
      <T icon={<Undo2 size={sz} />} title={t("app.undo")} run={() => c().undo().run()} disabled={!s.canUndo} />
      <T icon={<Redo2 size={sz} />} title={t("app.redo")} run={() => c().redo().run()} disabled={!s.canRedo} />
      <span className="sep" />
      <select className="style-select" value={styleValue} onChange={(e) => setStyle(e.target.value)} title={t("app.paragraphStyle")} aria-label={t("app.paragraphStyle")}>
        {!styleValue && <option value="">—</option>}
        <option value="p">{t("app.styleBodyText")}</option>
        {headingNames.map((n, i) => (
          <option key={n} value={`h${i + 1}`}>
            {n}
          </option>
        ))}
      </select>
      <span className="sep" />
      <T icon={<Bold size={sz} />} title={t("app.boldTitle")} on={s.bold} run={() => c().toggleBold().run()} />
      <T icon={<Italic size={sz} />} title={t("app.italicTitle")} on={s.italic} run={() => c().toggleItalic().run()} />
      <T icon={<Code size={sz} />} title={t("app.codeTitle")} on={s.code} run={() => c().toggleCode().run()} />
      <span className="sep" />
      <T icon={<List size={sz} />} title={t("app.bulletList")} on={s.bullet} run={() => ins().toggleBulletList().run()} />
      <T icon={<ListOrdered size={sz} />} title={t("app.numberedList")} on={s.ordered} run={() => ins().toggleOrderedList().run()} />
      <T icon={<TextQuote size={sz} />} title={t("app.quote")} on={s.quote} run={() => c().toggleBlockquote().run()} />
      <T
        icon={<ArrowRightLeft size={sz} />}
        title={t("app.textDirection")}
        on={!!s.otherDir}
        run={() => c().toggleDirection().run()}
      />
      <T icon={<AlignLeft size={sz} />} title={t("app.alignLeft")} on={s.align === "left"} disabled={!s.paragraph} run={() => c().toggleAlign("left").run()} />
      <T icon={<AlignCenter size={sz} />} title={t("app.alignCenter")} on={s.align === "center"} disabled={!s.paragraph} run={() => c().toggleAlign("center").run()} />
      <T icon={<AlignRight size={sz} />} title={t("app.alignRight")} on={s.align === "right"} disabled={!s.paragraph} run={() => c().toggleAlign("right").run()} />
      <span className="sep" />
      <T icon={<Superscript size={sz} />} title={t("app.inlineMath")} run={() => c().insertInlineMath().run()} />
      <T icon={<Sigma size={sz} />} title={t("app.equation")} run={() => ins().insertEquation().run()} />
      <T
        icon={<Image size={sz} />}
        title={t("app.figure")}
        run={() =>
          pickFigureImage().then((src) => {
            if (src) ins().insertFigure(src).run();
          }, onError)
        }
      />
      <T icon={<Table size={sz} />} title={t("app.tableWithCaption")} run={() => ins().insertCaptionedTable(3, 3).run()} />
      <T icon={<Braces size={sz} />} title={t("app.rawLatex")} run={() => ins().insertRawLatex().run()} />
      {deck && (
        <>
          <span className="sep" />
          <T icon={<GalleryVerticalEnd size={sz} />} label={t("app.slide")} title={t("app.slideTitle")} run={() => c().insertSlide().run()} />
          <T icon={<Columns2 size={sz} />} title={t("app.columns")} run={() => ins().insertColumns(2).run()} />
          <T icon={<SquareStack size={sz} />} title={t("app.slideBox")} run={() => ins().insertSlideBox("block").run()} />
          <T icon={<Pause size={sz} />} title={t("app.pause")} run={() => ins().insertPause().run()} />
          <T icon={<Palette size={sz} />} label={t("app.theme")} title={t("app.themeTitle")} run={onThemes} />
        </>
      )}
      <span className="sep" />
      <T icon={<Link2 size={sz} />} title={t("app.crossRef")} run={() => onPicker("ref")} />
      <T icon={<Quote size={sz} />} title={t("app.citation")} run={() => onPicker("cite")} />
      {!deck && <T icon={<BookMarked size={sz} />} title={t("app.appendices")} run={() => c().insertAppendices().run()} />}
      <span className="spacer" />
      {s.level > 0 && (
        <span className="context-tools" role="group" aria-label={t("app.heading")}>
          <T
            icon={<Hash size={15} />}
            label={t("app.numbered")}
            on={s.headingNumbered}
            title={t("app.numberedTitle")}
            run={() => c().toggleHeadingNumbered().run()}
          />
        </span>
      )}
      {s.inTable && (
        <span className="context-tools" role="group" aria-label={t("app.table")}>
          <T icon={<Rows3 size={15} />} label={t("app.addRow")} title={t("app.addRowTitle")} run={() => c().addRowAfter().run()} />
          <T icon={<Columns3 size={15} />} label={t("app.addCol")} title={t("app.addColTitle")} run={() => c().addColumnAfter().run()} />
          <T icon={<Rows3 size={15} />} label={t("app.deleteRow")} title={t("app.deleteRowTitle")} run={() => c().deleteRow().run()} />
          <T icon={<Columns3 size={15} />} label={t("app.deleteCol")} title={t("app.deleteColTitle")} run={() => c().deleteColumn().run()} />
          <T icon={<Table size={15} />} label={t("app.header")} title={t("app.headerTitle")} run={() => c().toggleHeaderRow().run()} />
          <T icon={<TableCellsMerge size={15} />} label={t("app.mergeNext")} title={t("app.mergeNextTitle")} run={() => c().mergeWithNeighbour("next").run()} disabled={!s.canMergeNext} />
          <T icon={<TableCellsMerge size={15} className="rotate-90" />} label={t("app.mergeBelow")} title={t("app.mergeBelowTitle")} run={() => c().mergeWithNeighbour("below").run()} disabled={!s.canMergeBelow} />
          <T icon={<TableCellsMerge size={15} />} label={t("app.merge")} title={t("app.mergeTitle")} run={() => c().mergeCells().run()} disabled={!s.canMerge} />
          <T icon={<TableCellsSplit size={15} />} label={t("app.split")} title={t("app.splitTitle")} run={() => c().splitCell().run()} disabled={!s.canSplit} />
          <T icon={<Trash2 size={15} />} title={t("app.deleteTable")} run={() => c().deleteCaptionedTable().run()} />
        </span>
      )}
    </div>
  );
}

function StatusBadge({ status, ms }: { status: Status; ms: number | null }) {
  const t = useT();
  if (status.state === "compiling")
    return (
      <span className="status status-compiling">
        <LoaderCircle size={13} className="spin" aria-hidden="true" /> {t("app.compiling")}
      </span>
    );
  if (status.state === "error")
    return (
      <span className="status status-error">
        <CircleAlert size={13} aria-hidden="true" />{" "}
        {status.error.kind === "texErrors"
          ? t(status.error.errors.length === 1 ? "app.latexErrorsOne" : "app.latexErrorsMany", { n: status.error.errors.length })
          : t("app.error")}
      </span>
    );
  if (status.state === "ok")
    return (
      <span className="status status-ok">
        <CircleCheck size={13} aria-hidden="true" /> {t("app.upToDate")}
        {ms != null && ` · ${t("app.seconds", { s: (ms / 1000).toFixed(1) })}`}
      </span>
    );
  return <span className="status">{t("app.ready")}</span>;
}

function TemplateProblemPanel({
  problem,
  onDismiss,
}: {
  problem: { path: string; issues: api.Issue[] };
  onDismiss: () => void;
}) {
  const t = useT();
  return (
    <div className="error-panel">
      <button className="dismiss" onClick={onDismiss} title={t("app.dismiss")} aria-label={t("app.dismiss")}>
        ×
      </button>
      <strong>{t("app.templateHasErrors")}</strong>
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
function errorHint(message: string): MessageKey | null {
  const m = message.toLowerCase();
  if (m.includes("undefined control sequence")) return "app.hintUndefinedCommand";
  if (m.includes("missing $ inserted")) return "app.hintMissingDollar";
  if (m.includes("missing } inserted") || m.includes("extra }") || m.includes("runaway argument")) return "app.hintBraces";
  if (m.includes("not found") && (m.includes("file") || m.includes(".png") || m.includes(".jpg"))) return "app.hintFileMissing";
  if (m.includes("misplaced alignment tab")) return "app.hintAlignmentTab";
  if (m.includes("there's no line here to end")) return "app.hintNoLine";
  return null;
}

function ErrorPanel({
  error,
  onJump,
  describe,
}: {
  error: AppError;
  /** Goes to a block (in a project: of `chapter`). */
  onJump?: (block: number, chapter?: string) => void;
  describe?: (block: number, chapter?: string) => string;
}) {
  const t = useT();
  if (error.kind === "texErrors") {
    return (
      <div className="error-panel">
        <strong>{t("app.latexReportedErrors")}</strong>
        <ul className="error-list">
          {error.errors.map((e, i) => {
            const hint = errorHint(e.message);
            const body = (
              <>
                <span className="error-message">{e.message}</span>
                {hint && <span className="error-hint">{t(hint)}</span>}
                {e.block != null && describe && <span className="error-where">{t("app.errorIn", { where: describe(e.block, e.chapter) })}</span>}
              </>
            );
            return (
              <li key={i}>
                {e.block != null && onJump ? (
                  <button type="button" className="error-jump" onClick={() => onJump(e.block!, e.chapter)} title={t("app.goToPlace")}>
                    {body}
                  </button>
                ) : (
                  <div className="error-item">
                    {e.line != null && <span className="muted">{t("app.errorLine", { n: e.line })}</span>}
                    {body}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
        <details>
          <summary>{t("app.fullLog")}</summary>
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
      <strong>{error.kind === "tectonicMissing" ? t("app.engineNotAvailable") : t("app.error")}:</strong> {error.message}
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
