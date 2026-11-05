// Typed wrappers around the Rust commands in src-tauri/src/lib.rs.
import { invoke } from "@tauri-apps/api/core";
import type { JSONContent } from "@tiptap/react";

export interface TexError {
  file: string | null;
  line: number | null;
  message: string;
}

export type AppError =
  | { kind: "tectonicMissing"; message: string }
  | { kind: "texErrors"; errors: TexError[]; log: string }
  | { kind: "other"; message: string };

/** Turns anything thrown by invoke() into an AppError. */
export function asAppError(e: unknown): AppError {
  if (e && typeof e === "object" && "kind" in e) return e as AppError;
  return { kind: "other", message: String(e) };
}

export const renderPdf = (doc: JSONContent) => invoke<ArrayBuffer>("render_pdf", { doc });
export const toLatex = (doc: JSONContent) => invoke<string>("to_latex", { doc });
export const saveDocument = (path: string, doc: JSONContent) => invoke<void>("save_document", { path, doc });
export const openDocument = (path: string) => invoke<JSONContent>("open_document", { path });
export const exportTex = (path: string, doc: JSONContent) => invoke<void>("export_tex", { path, doc });
