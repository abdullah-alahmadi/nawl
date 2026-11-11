// Typed wrappers around the Rust commands in src-tauri/src/lib.rs.
// Template/field types are generated from Rust by ts-rs (src/bindings, `cargo test`).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { JSONContent } from "@tiptap/react";
import type { Template } from "./bindings/Template";
import type { FieldValue } from "./bindings/FieldValue";
import type { Issue } from "./bindings/Issue";
import type { ProjectEntry } from "./bindings/ProjectEntry";
import type { ProjectIndex } from "./bindings/ProjectIndex";
import type { ChapterIndex } from "./bindings/ChapterIndex";
import type { IndexTarget } from "./bindings/IndexTarget";

export type { Template, FieldValue, Issue };
export type Metadata = Record<string, FieldValue>;

export interface TemplateInfo {
  template: Template;
  /** The template.json in use; null for the built-in template or a saved copy. */
  path: string | null;
  warnings: Issue[];
}

export interface BibEntry {
  key: string;
  kind: string;
  authors: string[];
  year: string | null;
  title: string;
  /** The authors in full (given and family names), for editing. */
  people: { given: string; family: string }[];
  /** Every field as written in the BibTeX, for editing. */
  fields: Record<string, string>;
}

export interface BibInfo {
  entries: BibEntry[];
  error: string | null;
}

export interface OpenedDocument {
  doc: JSONContent;
  metadata: Metadata;
  /** BibTeX source. */
  bibliography: string;
  /** The template the document was saved with, now active (absent for old files). */
  template?: TemplateInfo | null;
  /** A single-file document (images inside); saving keeps it one. */
  bundled?: boolean;
  /** The file was a chapter of a project: the project file, to open instead. */
  project?: string;
}

export type TemplateReloadEvent =
  | { kind: "reloaded"; info: TemplateInfo }
  | { kind: "invalid"; path: string; issues: Issue[] };

export interface TexError {
  file: string | null;
  line: number | null;
  message: string;
  /** The top-level block of the document whose LaTeX failed, when known. */
  block?: number | null;
  /** In a project: the chapter file the block is in. */
  chapter?: string;
}

export type AppError =
  | { kind: "tectonicMissing"; message: string }
  | { kind: "texErrors"; errors: TexError[]; log: string }
  | { kind: "templateInvalid"; path: string; issues: Issue[] }
  | { kind: "other"; message: string };

/** Turns anything thrown by invoke() into an AppError. */
export function asAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "kind" in e) return e as AppError;
  return { kind: "other", message: String(e) };
}

export const getTemplate = () => invoke<TemplateInfo>("get_template");
export const openTemplate = (path: string) => invoke<TemplateInfo>("open_template", { path });
export interface BuiltinTemplate {
  id: string;
  name: string;
  description: string;
  /** Publisher class it's built on (e.g. "IEEEtran"); null for Nawl layouts. */
  class: string | null;
  kind: "article" | "presentation";
  /** The document language: Arabic templates are listed together. */
  language: "english" | "arabic";
  /** Sections at the top (articles) or chapters (reports, theses). */
  topLevel: "chapter" | "section";
}
export const listBuiltinTemplates = () => invoke<BuiltinTemplate[]>("list_builtin_templates");
export const switchToBuiltinTemplate = (id?: string) => invoke<TemplateInfo>("use_builtin_template", { id: id ?? null });
/** A journal template for a class of the installed TeX distribution. */
export interface InstalledTemplate {
  class: string;
  name: string;
  description: string;
  /** TeX Live package the class comes from. */
  package: string;
  /** Fails under XeLaTeX and LuaLaTeX (and Tectonic). */
  pdflatexOnly: boolean;
  category: InstalledCategory;
}
export type InstalledCategory = "journal" | "thesis" | "book" | "general" | "letter" | "cv" | "poster" | "exam" | "presentation";
/** What the installed TeX offers: templates, and how many classes of kinds Nawl
 *  can't write yet are installed. */
export interface InstalledList {
  templates: InstalledTemplate[];
  waiting: { category: InstalledCategory; count: number }[];
}
export const listInstalledTemplates = () => invoke<InstalledList>("list_installed_templates");
export const switchToInstalledTemplate = (cls: string) => invoke<TemplateInfo>("use_installed_template", { class: cls });
/** Beamer themes of the installed TeX distribution (empty without one). */
export interface BeamerThemeList {
  themes: string[];
  colorThemes: string[];
  fontThemes: string[];
}
export const listBeamerThemes = () => invoke<BeamerThemeList>("list_beamer_themes");
/** A two-slide PDF showing a theme (cached by the backend). `installed`: only the
 *  installed TeX has it, so it's rendered with pdfLaTeX when the engine is Tectonic. */
export const themePreview = (theme: string, colorTheme: string, fontTheme: string, installed: boolean) =>
  invoke<ArrayBuffer>("theme_preview", { theme, colorTheme, fontTheme, installed });
/** Copies a publisher kit file (.cls, .sty, .bst…) into the template's assets/. */
export const importTemplateFile = (source: string) => invoke<string>("import_template_file", { source });
/** Imports a publisher's kit .zip into the template's assets/; returns the files. */
export const importKitZip = (source: string) => invoke<string[]>("import_kit_zip", { source });
/** Fires when the open template file changes on disk. Returns an unsubscribe function. */
export const onTemplateReloaded = (cb: (e: TemplateReloadEvent) => void) =>
  listen<TemplateReloadEvent>("template-reloaded", (event) => cb(event.payload));
/** Validates the designer's draft and makes it active; rejects with templateInvalid. */
export const setTemplate = (template: Template) => invoke<Issue[]>("set_template", { template });
export const saveTemplate = (path: string, template: Template) =>
  invoke<TemplateInfo>("save_template", { path, template });
/** What a blank template starts as. */
export interface BlankSpec {
  presentation: boolean;
  paper: { kind: "a4" | "letter" };
  /** Chapters (reports) instead of sections (articles). */
  chapters: boolean;
  /** Arabic: right to left, Arabic labels, English as the second language. */
  arabic: boolean;
}
/** Creates a template from scratch in My templates and makes it active. */
export const createBlankTemplate = (name: string, spec: BlankSpec) => invoke<TemplateInfo>("create_blank_template", { name, spec });
/** Saves the template into My templates as a new template called `name` and makes it active. */
export const saveTemplateToLibrary = (name: string, template: Template) =>
  invoke<TemplateInfo>("save_template_to_library", { name, template });
export const importTemplateAsset = (source: string) => invoke<string>("import_template_asset", { source });
export const readTemplateAsset = (asset: string) => invoke<ArrayBuffer>("read_template_asset", { asset });
export const importFigure = (source: string) => invoke<string>("import_figure", { source });
export const readDocumentFile = (path: string) => invoke<ArrayBuffer>("read_document_file", { path });
export const exportPdf = (path: string, d: DocState) => invoke<void>("export_pdf", { path, ...d });
export const listSystemFonts = () => invoke<string[]>("list_system_fonts");
/** Installed font families that have Arabic letters (Diwani, Thuluth…). */
export const listArabicSystemFonts = () => invoke<string[]>("list_arabic_system_fonts");
/** A family of Nawl's Arabic font library. */
export interface LibraryFont {
  id: string;
  name: string;
  /** naskh, sans, kufi, calligraphic, display or nastaliq. */
  category: string;
  hasBold: boolean;
  /** Its files are there (a development checkout may not have them). */
  available: boolean;
}
export const listLibraryFonts = () => invoke<LibraryFont[]>("list_library_fonts");
export const readLibraryFont = (id: string, bold: boolean) => invoke<ArrayBuffer>("read_library_font", { id, bold });
/** An example document: the senior design report, or "presentation". */
export const sampleDocument = (name?: "presentation" | "arabic-article" | "arabic-presentation") => invoke<OpenedDocument>("sample_document", { name: name ?? null });
/** Everything about the open document that the backend needs to generate it. */
export interface DocState {
  doc: JSONContent;
  metadata: Metadata;
  bibliography: string;
}

export const checkDocument = (d: DocState) => invoke<Issue[]>("check_document", { ...d });
/** A place in the PDF, in PDF points from the page's top left (SyncTeX). */
export interface PdfSpot {
  /** 1-based. */
  page: number;
  x: number;
  y: number;
  height: number;
}
/** The top-level block behind a point of the last compiled PDF, if known. */
export const pdfToBlock = (page: number, x: number, y: number) => invoke<number | null>("pdf_to_block", { page, x, y });
/** Where a top-level block is in the last compiled PDF, if known. */
export const blockToPdf = (block: number) => invoke<PdfSpot | null>("block_to_pdf", { block });
export const renderPdf = (d: DocState) => invoke<ArrayBuffer>("render_pdf", { ...d });
export const toLatex = (d: DocState) => invoke<string>("to_latex", { ...d });
/** Saves a document. `bundled`: one file with its images inside (true), a .nawl with
 *  the images in a folder next to it (false), or the format the file already has. */
export const saveDocument = (path: string, d: DocState, bundled?: boolean) => invoke<void>("save_document", { path, ...d, bundled });
export const parseBibliography = (source: string) => invoke<BibInfo>("parse_bibliography", { source });
export const readBibFile = (path: string) => invoke<string>("read_bib_file", { path });
export const newDocument = () => invoke<OpenedDocument>("new_document");
export const openDocument = (path: string) => invoke<OpenedDocument>("open_document", { path });
export const exportTex = (path: string, d: DocState) => invoke<void>("export_tex", { path, ...d });
/** Writes a Word file; `mathml` maps each formula's LaTeX to its MathML. */
export const exportDocx = (path: string, d: DocState, mathml: Record<string, string>, pictures: Record<string, FormulaPicture> = {}) =>
  invoke<void>("export_docx", { path, ...d, mathml, pictures });
/** A formula's picture for the Word file: PNG bytes and its size in TeX points
 *  (`depthPt` of the height is below the baseline). */
export interface FormulaPicture {
  png: number[];
  widthPt: number;
  heightPt: number;
  depthPt: number;
}
/** Arabic formulas set by the active template, one per PDF page exactly its size;
 *  `sizes`: each page's width, height above the baseline and depth, in TeX points. */
export const formulaPictures = (formulas: { latex: string; display: boolean }[]) =>
  invoke<{ pdf: number[]; sizes: [number, number, number][] }>("formula_pictures", { formulas });

// Crash recovery: unsaved changes are kept in the app's data folder.
export interface RecoveryInfo {
  /** The file the changes belong to; null for a document that was never saved. */
  originalPath: string | null;
  /** Seconds since 1970. */
  savedAt: number;
  /** For a chapter of a project: the chapter file (`originalPath` is the project). */
  chapter?: string;
}
export const writeRecovery = (originalPath: string | null, d: DocState, chapter?: string) =>
  invoke<void>("write_recovery", { originalPath, chapter: chapter ?? null, ...d });
export const recoveryInfo = () => invoke<RecoveryInfo | null>("recovery_info");
export const restoreRecovery = () =>
  invoke<OpenedDocument & { originalPath: string | null; chapter: string | null }>("restore_recovery");
export const discardRecovery = () => invoke<void>("discard_recovery");

export const getSettings = () => invoke<unknown>("get_settings");
/** The language of the backend's messages (errors, problems): the interface's. */
export const setMessageLanguage = (arabic: boolean) => invoke<void>("set_message_language", { arabic });
export const saveSettings = (settings: unknown) => invoke<void>("save_settings", { settings });

/** A template in the user's library ("My templates"). */
export interface UserTemplate {
  id: string;
  name: string;
  description: string;
  class: string | null;
  /** The template file. */
  path: string;
}
export const listUserTemplates = () => invoke<UserTemplate[]>("list_user_templates");
/** Imports a template (.json), a template package or a publisher kit (.zip) into the library. */
export const importTemplate = (source: string) => invoke<TemplateInfo>("import_template", { source });
export const removeUserTemplate = (id: string) => invoke<void>("remove_user_template", { id });
export const exportTemplate = (path: string) => invoke<void>("export_template", { path });

/** A TeX engine Nawl can compile with. */
export interface EngineInfo {
  /** "tectonic", "pdflatex", "xelatex" or "lualatex". */
  id: string;
  available: boolean;
  /** Version, or the path, when available. */
  detail: string | null;
  /** Installed engines run through latexmk when it's there. */
  latexmk: boolean;
}
export const listEngines = () => invoke<EngineInfo[]>("list_engines");
export const setEngine = (id: string) => invoke<void>("set_engine", { id });

// Projects: a book written as one .nawl per chapter (src-tauri/src/project.rs).
export type { ProjectEntry, ProjectIndex, ChapterIndex, IndexTarget };
export interface OpenedProject {
  path: string;
  metadata: Metadata;
  bibliography: string;
  chapters: ProjectEntry[];
  /** The project's template, now active. */
  template: TemplateInfo | null;
  index: ProjectIndex;
  /** Chapters that couldn't be read, and other problems. */
  issues: Issue[];
}
/** The open chapter and the book-wide values, sent with compiles and exports. */
export interface ProjectState {
  /** The chapter open in the editor, and its live content. */
  file: string | null;
  doc: JSONContent | null;
  metadata: Metadata;
  bibliography: string;
}
export interface ProjectSpot {
  file: string;
  block: number;
}
export const openProject = (path: string) => invoke<OpenedProject>("open_project", { path });
export const closeProject = () => invoke<void>("close_project");
export const readChapter = (file: string) => invoke<JSONContent>("read_chapter", { file });
export const saveChapter = (file: string, doc: JSONContent) => invoke<void>("save_chapter", { file, doc });
export const saveProject = (metadata: Metadata, bibliography: string, chapters: ProjectEntry[]) =>
  invoke<ProjectIndex>("save_project", { metadata, bibliography, chapters });
export const newChapter = (title: string, appendix: boolean) => invoke<string>("new_chapter", { title, appendix });
export const projectIndex = (file: string | null, doc: JSONContent | null) => invoke<ProjectIndex>("project_index", { file, doc });
/** Compiles the project: just the open chapter when `onlyChapter` and the numbers
 *  can't be out of date, else the whole book. */
export const renderProject = (s: ProjectState, onlyChapter: boolean) => invoke<ArrayBuffer>("render_project", { ...s, onlyChapter });
export const exportProjectPdf = (path: string, s: ProjectState) => invoke<void>("export_project_pdf", { path, ...s });
export const exportProjectTex = (path: string, s: ProjectState) => invoke<void>("export_project_tex", { path, ...s });
/** The whole book as one Word file; `mathml` covers every chapter's formulas. */
export const exportProjectDocx = (path: string, s: ProjectState, mathml: Record<string, string>, pictures: Record<string, FormulaPicture> = {}) =>
  invoke<void>("export_project_docx", { path, ...s, mathml, pictures });
export const projectPdfToBlock = (page: number, x: number, y: number) => invoke<ProjectSpot | null>("project_pdf_to_block", { page, x, y });
export const projectBlockToPdf = (file: string, block: number) => invoke<PdfSpot | null>("project_block_to_pdf", { file, block });
/** Splits the document into a project in `folder`; returns the project file. */
export const splitIntoProject = (folder: string, name: string, d: DocState) => invoke<string>("split_into_project", { folder, name, ...d });
/** Writes an example project (the Arabic thesis, else the senior design report) to a
 *  temporary folder; returns its path, for `openProject`. */
export const sampleProject = (name?: "arabic-thesis") => invoke<string>("sample_project", { name: name ?? null });
/** Writes the whole project as one document at `path`. */
export const joinProject = (path: string, s: ProjectState) => invoke<void>("join_project", { path, ...s });
