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

/** Placeholder names in a text (`{title}`, `{team}`…); `{{` and `}}` are literal braces. */
export function placeholderNames(text: string): string[] {
  return [...text.replace(/\{\{|\}\}/g, "").matchAll(/\{([a-z][a-z0-9_]*)\}/g)].map((m) => m[1]);
}

/** Where a Texloom layout shows each field: the title page, headers and footers,
 *  front-matter pages (the abstract), the raw preamble. Publisher classes and
 *  presentations write fields into their own title blocks by id, so there every field
 *  counts as used ("the class's title block"). */
export function fieldUses(t: Template): Map<string, string[]> {
  const uses = new Map<string, string[]>();
  const note = (id: string, place: string) => {
    const list = uses.get(id) ?? [];
    if (!list.includes(place)) uses.set(id, [...list, place]);
  };
  if (t.base || t.kind === "presentation") {
    for (const f of t.fields) note(f.id, t.kind === "presentation" ? "the title slide" : "the class's title block");
    return uses;
  }
  const scan = (text: string | undefined, place: string) => placeholderNames(text ?? "").forEach((n) => note(n, place));
  for (const item of t.titlePage.items) if (item.content.type === "text") scan(item.content.text, "the title page");
  // Parts left out of a template file are empty (the backend fills them in, a raw file may not).
  const hf = t.headerFooter ?? {};
  for (const style of [hf.frontMatter, hf.main, hf.chapterOpening]) {
    for (const s of [style?.header?.left, style?.header?.center, style?.header?.right]) scan(s, "the page header");
    for (const s of [style?.footer?.left, style?.footer?.center, style?.footer?.right]) scan(s, "the page footer");
  }
  for (const item of t.frontMatter) {
    if (item.type === "fieldPage") note(item.field, `the “${item.title}” page`);
    if (item.type === "fixedPage")
      for (const it of item.items) if (it.content.type === "text") scan(it.content.text, `the “${item.title}” page`);
  }
  for (const f of t.fields) if (t.rawPreamble && new RegExp(`\\b${f.id}\\b`).test(t.rawPreamble)) note(f.id, "the raw preamble");
  return uses;
}

/** Field ids the template still shows somewhere (see `fieldUses`). */
export function usedFieldIds(t: Template): Set<string> {
  return new Set(fieldUses(t).keys());
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
