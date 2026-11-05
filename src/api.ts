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

export interface OpenedDocument {
  doc: JSONContent;
  metadata: Metadata;
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
export const switchToBuiltinTemplate = () => invoke<TemplateInfo>("use_builtin_template");
/** Fires when the open template file changes on disk. Returns an unsubscribe function. */
export const onTemplateReloaded = (cb: (e: TemplateReloadEvent) => void) =>
  listen<TemplateReloadEvent>("template-reloaded", (event) => cb(event.payload));
/** Validates the designer's draft and makes it active; rejects with templateInvalid. */
export const setTemplate = (template: Template) => invoke<Issue[]>("set_template", { template });
export const saveTemplate = (path: string, template: Template) =>
  invoke<TemplateInfo>("save_template", { path, template });
export const importTemplateAsset = (source: string) => invoke<string>("import_template_asset", { source });
export const readTemplateAsset = (asset: string) => invoke<ArrayBuffer>("read_template_asset", { asset });
export const listSystemFonts = () => invoke<string[]>("list_system_fonts");
export const sampleDocument = () => invoke<OpenedDocument>("sample_document");
export const checkMetadata = (metadata: Metadata) => invoke<Issue[]>("check_metadata", { metadata });
export const renderPdf = (doc: JSONContent, metadata: Metadata) => invoke<ArrayBuffer>("render_pdf", { doc, metadata });
export const toLatex = (doc: JSONContent, metadata: Metadata) => invoke<string>("to_latex", { doc, metadata });
export const saveDocument = (path: string, doc: JSONContent, metadata: Metadata) =>
  invoke<void>("save_document", { path, doc, metadata });
export const openDocument = (path: string) => invoke<OpenedDocument>("open_document", { path });
export const exportTex = (path: string, doc: JSONContent, metadata: Metadata) =>
  invoke<void>("export_tex", { path, doc, metadata });
