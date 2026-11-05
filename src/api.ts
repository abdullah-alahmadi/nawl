// Typed wrappers around the Rust commands in src-tauri/src/lib.rs.
// Template/field types are generated from Rust by ts-rs (src/bindings, `cargo test`).
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { JSONContent } from "@tiptap/react";
import type { Template } from "./bindings/Template";
import type { FieldValue } from "./bindings/FieldValue";
import type { Issue } from "./bindings/Issue";

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
}

export type TemplateReloadEvent =
  | { kind: "reloaded"; info: TemplateInfo }
  | { kind: "invalid"; path: string; issues: Issue[] };

export interface TexError {
  file: string | null;
  line: number | null;
  message: string;
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
  /** Publisher class it's built on (e.g. "IEEEtran"); null for Texloom layouts. */
  class: string | null;
}
export const listBuiltinTemplates = () => invoke<BuiltinTemplate[]>("list_builtin_templates");
export const switchToBuiltinTemplate = (id?: string) => invoke<TemplateInfo>("use_builtin_template", { id: id ?? null });
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
export const importTemplateAsset = (source: string) => invoke<string>("import_template_asset", { source });
export const readTemplateAsset = (asset: string) => invoke<ArrayBuffer>("read_template_asset", { asset });
export const importFigure = (source: string) => invoke<string>("import_figure", { source });
export const readDocumentFile = (path: string) => invoke<ArrayBuffer>("read_document_file", { path });
export const exportPdf = (path: string, d: DocState) => invoke<void>("export_pdf", { path, ...d });
export const listSystemFonts = () => invoke<string[]>("list_system_fonts");
export const sampleDocument = () => invoke<OpenedDocument>("sample_document");
/** Everything about the open document that the backend needs to generate it. */
export interface DocState {
  doc: JSONContent;
  metadata: Metadata;
  bibliography: string;
}

export const checkDocument = (d: DocState) => invoke<Issue[]>("check_document", { ...d });
export const renderPdf = (d: DocState) => invoke<ArrayBuffer>("render_pdf", { ...d });
export const toLatex = (d: DocState) => invoke<string>("to_latex", { ...d });
export const saveDocument = (path: string, d: DocState) => invoke<void>("save_document", { path, ...d });
export const parseBibliography = (source: string) => invoke<BibInfo>("parse_bibliography", { source });
export const readBibFile = (path: string) => invoke<string>("read_bib_file", { path });
export const newDocument = () => invoke<OpenedDocument>("new_document");
export const openDocument = (path: string) => invoke<OpenedDocument>("open_document", { path });
export const exportTex = (path: string, d: DocState) => invoke<void>("export_tex", { path, ...d });

// Crash recovery: unsaved changes are kept in the app's data folder.
export interface RecoveryInfo {
  /** The file the changes belong to; null for a document that was never saved. */
  originalPath: string | null;
  /** Seconds since 1970. */
  savedAt: number;
}
export const writeRecovery = (originalPath: string | null, d: DocState) =>
  invoke<void>("write_recovery", { originalPath, ...d });
export const recoveryInfo = () => invoke<RecoveryInfo | null>("recovery_info");
export const restoreRecovery = () =>
  invoke<OpenedDocument & { originalPath: string | null }>("restore_recovery");
export const discardRecovery = () => invoke<void>("discard_recovery");

export const getSettings = () => invoke<unknown>("get_settings");
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

/** A TeX engine Texloom can compile with. */
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
