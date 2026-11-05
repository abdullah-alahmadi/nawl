import type { Issue, Template } from "../api";
import type { PlaceholderOption } from "./controls";

/** Applies a mutation to a copy of the template (the designer's only way to change it). */
export type Edit = (fn: (t: Template) => void) => void;

export interface SectionProps {
  t: Template;
  edit: Edit;
  issues: Issue[];
}

export const DYNAMIC_PLACEHOLDERS: PlaceholderOption[] = [
  { name: "page", description: "Current page number" },
  { name: "pages", description: "Total number of pages" },
  { name: "chapter", description: "Current chapter title (section title without chapters)" },
  { name: "section", description: "Current section title (subsection title without chapters)" },
];

export function fieldPlaceholders(t: Template, kinds: string[] = ["text", "list"]): PlaceholderOption[] {
  return t.fields.filter((f) => kinds.includes(f.kind)).map((f) => ({ name: f.id, description: f.label }));
}

/** Heading names for editor levels 1–4 under the template's structure. */
export function headingNames(t: Template): string[] {
  return t.structure.topLevel === "chapter"
    ? ["Chapter", "Section", "Subsection", "Subsubsection"]
    : ["Section", "Subsection", "Subsubsection", "Paragraph"];
}

/** Lowercase placeholder-safe id from a label, unique among `taken`. */
export function slugId(label: string, taken: string[]): string {
  let base = label
    .toLowerCase()
    .normalize("NFKD")
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
  if (!/^[a-z]/.test(base)) base = "field" + (base ? "_" + base : "");
  let id = base;
  for (let n = 2; taken.includes(id); n++) id = `${base}_${n}`;
  return id;
}
