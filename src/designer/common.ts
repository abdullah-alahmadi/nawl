import type { Issue, Template } from "../api";
import type { PlaceholderOption } from "./controls";
import { t as tr, type MessageKey } from "../i18n";

/** `t` from `useT()`: section components name it `tr`, since `t` is the template. */
export type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string;

/** Applies a mutation to a copy of the template (the designer's only way to change it). */
export type Edit = (fn: (t: Template) => void) => void;

export interface SectionProps {
  t: Template;
  edit: Edit;
  issues: Issue[];
}

export function dynamicPlaceholders(tr: Translate): PlaceholderOption[] {
  return [
    { name: "page", description: tr("designer.phPage") },
    { name: "pages", description: tr("designer.phPages") },
    { name: "chapter", description: tr("designer.phChapter") },
    { name: "section", description: tr("designer.phSection") },
  ];
}

/** What an authors field gives besides the names (`{authors_affiliations}`…), as in
 *  `nawl_core::AUTHOR_SUFFIXES`, with each one's description key. */
export const AUTHOR_SUFFIXES: [string, MessageKey][] = [
  ["_affiliations", "designer.authorsAffiliations"],
  ["_en", "designer.authorsEnglish"],
  ["_affiliations_en", "designer.authorsAffiliationsEnglish"],
  ["_email", "designer.authorsEmail"],
];

/** The field a derived placeholder belongs to (`date_hijri` → `date`, `authors_en` →
 *  `authors`), or the name itself. */
export function placeholderField(t: Template, name: string): string {
  if (name.endsWith("_hijri") && t.fields.some((f) => f.id === name.slice(0, -6) && f.kind === "date")) return name.slice(0, -6);
  for (const [suffix] of [...AUTHOR_SUFFIXES].sort((a, b) => b[0].length - a[0].length)) {
    const base = name.slice(0, -suffix.length);
    if (name.endsWith(suffix) && t.fields.some((f) => f.id === base && f.kind === "authors")) return base;
  }
  return name;
}

export function fieldPlaceholders(t: Template, kinds: string[] = ["text", "list"]): PlaceholderOption[] {
  // Date fields go wherever text does, with their Hijri form beside them; authors
  // fields on title pages, with their affiliations, English names and e-mail.
  const authors = kinds.includes("list");
  return t.fields
    .filter((f) => kinds.includes(f.kind) || (f.kind === "date" && kinds.includes("text")) || (f.kind === "authors" && authors))
    .flatMap((f) =>
      f.kind === "date"
        ? [
            { name: f.id, description: f.label },
            { name: f.id + "_hijri", description: tr("designer.hijriOf", { label: f.label }) },
          ]
        : f.kind === "authors"
          ? [{ name: f.id, description: f.label }, ...AUTHOR_SUFFIXES.map(([s, key]) => ({ name: f.id + s, description: tr(key, { label: f.label }) }))]
          : [{ name: f.id, description: f.label }],
    );
}

/** Placeholder names in a text (`{title}`, `{team}`…); `{{` and `}}` are literal braces. */
export function placeholderNames(text: string): string[] {
  return [...text.replace(/\{\{|\}\}/g, "").matchAll(/\{([a-z][a-z0-9_]*)\}/g)].map((m) => m[1]);
}

/** Where a Nawl layout shows each field: the title page, headers and footers,
 *  front-matter pages (the abstract), the raw preamble. Publisher classes and
 *  presentations write fields into their own title blocks by id, so there every field
 *  counts as used ("the class's title block"). The places are said in the interface
 *  language (callers put them into sentences, sometimes outside render). */
export function fieldUses(t: Template): Map<string, string[]> {
  const uses = new Map<string, string[]>();
  const note = (id: string, place: string) => {
    const list = uses.get(id) ?? [];
    if (!list.includes(place)) uses.set(id, [...list, place]);
  };
  if (t.base || t.kind === "presentation") {
    for (const f of t.fields) note(f.id, tr(t.kind === "presentation" ? "designer.usedTitleSlide" : "designer.usedClassTitleBlock"));
    return uses;
  }
  // {date_hijri} uses the date field, {authors_en} the authors field.
  const scan = (text: string | undefined, place: string) => placeholderNames(text ?? "").forEach((n) => note(placeholderField(t, n), place));
  for (const item of t.titlePage.items) if (item.content.type === "text") scan(item.content.text, tr("designer.usedTitlePage"));
  // Parts left out of a template file are empty (the backend fills them in, a raw file may not).
  const hf = t.headerFooter ?? {};
  for (const style of [hf.frontMatter, hf.main, hf.chapterOpening]) {
    for (const s of [style?.header?.left, style?.header?.center, style?.header?.right]) scan(s, tr("designer.usedHeader"));
    for (const s of [style?.footer?.left, style?.footer?.center, style?.footer?.right]) scan(s, tr("designer.usedFooter"));
  }
  for (const item of t.frontMatter) {
    if (item.type === "fieldPage") note(item.field, tr("designer.usedNamedPage", { title: item.title }));
    if (item.type === "fixedPage") {
      const page = tr("designer.usedNamedPage", { title: item.title });
      for (const it of item.items) if (it.content.type === "text") scan(it.content.text, page);
    }
  }
  for (const f of t.fields) if (t.rawPreamble && new RegExp(`\\b${f.id}\\b`).test(t.rawPreamble)) note(f.id, tr("designer.usedRawPreamble"));
  return uses;
}

/** Field ids the template still shows somewhere (see `fieldUses`). */
export function usedFieldIds(t: Template): Set<string> {
  return new Set(fieldUses(t).keys());
}

export type HeadingLevel = "Chapter" | "Section" | "Subsection" | "Subsubsection" | "Paragraph";

/** Heading levels for editor levels 1–4 under the template's structure; names and
 *  "down to" depths are `designer.level…` / `designer.depth…` keys. */
export function headingLevels(t: Template): HeadingLevel[] {
  return t.structure.topLevel === "chapter"
    ? ["Chapter", "Section", "Subsection", "Subsubsection"]
    : ["Section", "Subsection", "Subsubsection", "Paragraph"];
}

/** Heading names for editor levels 1–4, in the interface language. */
export function headingNames(t: Template, tr: Translate): string[] {
  return headingLevels(t).map((l) => tr(`designer.level${l}`));
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
