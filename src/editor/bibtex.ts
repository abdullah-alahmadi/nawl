// Writing BibTeX entries from the reference window, and finding an entry in the
// bibliography source so it can be replaced or removed.

export interface Person {
  given: string;
  /** Family name, or an organisation's whole name (when `given` is empty). */
  family: string;
}

export interface RefDraft {
  kind: string;
  key: string;
  people: Person[];
  /** BibTeX field → value as the user typed it (plain text). */
  fields: Record<string, string>;
}

/** LaTeX's special characters in plain text; URLs and DOIs stay as typed. */
export function escapeField(name: string, value: string): string {
  const v = value.trim().replace(/\s+/g, " ");
  if (name === "url" || name === "doi") return v.replace(/[{}]/g, "");
  return v.replace(/\\?([&%#_$])/g, "\\$1");
}

/** The reverse, for showing a stored value in the form. */
export function unescapeField(value: string): string {
  return value.replace(/\\([&%#_$])/g, "$1");
}

function personText(p: Person): string {
  const family = p.family.trim();
  const given = p.given.trim();
  if (!family) return "";
  // An organisation: braces keep BibTeX from splitting it into first/last names.
  if (!given) return family.includes(" ") ? `{${escapeField("", family)}}` : escapeField("", family);
  return `${escapeField("", family)}, ${escapeField("", given)}`;
}

/** The entry as BibTeX text. */
export function toBibtex(r: RefDraft): string {
  const lines: string[] = [];
  const authors = r.people.map(personText).filter(Boolean);
  if (authors.length) lines.push(`  author = {${authors.join(" and ")}}`);
  for (const [name, value] of Object.entries(r.fields)) {
    if (!value.trim()) continue;
    lines.push(`  ${name} = {${escapeField(name, value)}}`);
  }
  return `@${r.kind}{${r.key},\n${lines.join(",\n")}\n}`;
}

/** ASCII letters and digits only, lower case: "Müller" → "muller". */
function slug(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

const SMALL_WORDS = new Set(["a", "an", "the", "of", "on", "in", "for", "and", "to", "with", "at", "by", "from"]);

/** A citation key like "smith2021sensor", unique among `taken`. */
export function makeKey(r: RefDraft, taken: Set<string>): string {
  const first = r.people.find((p) => p.family.trim());
  // A person's last family-name word ("van Gogh" → "gogh"); an organisation's first word.
  const words = first ? first.family.trim().split(/\s+/) : [];
  const name = first ? slug((first.given.trim() ? words[words.length - 1] : words[0]) ?? "") : "ref";
  const year = (r.fields.year ?? "").match(/\d{4}/)?.[0] ?? "";
  const word = (r.fields.title ?? "")
    .split(/\s+/)
    .map(slug)
    .find((w) => w && !SMALL_WORDS.has(w));
  const base = `${name || "ref"}${year}${word ?? ""}` || "ref";
  if (!taken.has(base)) return base;
  for (let i = 0; i < 26; i++) {
    const k = `${base}${String.fromCharCode(97 + i)}`;
    if (!taken.has(k)) return k;
  }
  return `${base}${taken.size}`;
}

/** Keys BibTeX and Texloom accept. */
export function validKey(key: string): boolean {
  return /^[A-Za-z0-9_:.\-/+]+$/.test(key);
}

/** Where the entry with `key` sits in `source`: [start, end), or null. */
export function findEntry(source: string, key: string): [number, number] | null {
  const escaped = key.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&");
  const head = new RegExp(`@\\s*[A-Za-z]+\\s*([{(])\\s*${escaped}\\s*,`, "g");
  const m = head.exec(source);
  if (!m) return null;
  const open = m.index + m[0].indexOf(m[1]);
  const close = m[1] === "{" ? "}" : ")";
  let depth = 0;
  for (let i = open; i < source.length; i++) {
    const ch = source[i];
    if (ch === "\\") {
      i++;
      continue;
    }
    if (ch === "{" || (ch === "(" && m[1] === "(")) depth++;
    else if (ch === "}" || (ch === ")" && close === ")")) {
      depth--;
      if (depth === 0) return [m.index, i + 1];
    }
  }
  return null;
}

/** `source` with the entry `key` replaced by `text` (or removed when `text` is null). */
export function replaceEntry(source: string, key: string, text: string | null): string {
  const at = findEntry(source, key);
  if (!at) return text === null ? source : appendEntry(source, text);
  const [start, end] = at;
  if (text !== null) return source.slice(0, start) + text + source.slice(end);
  // Removing: take the blank lines around it too.
  const before = source.slice(0, start).replace(/\s*$/, "");
  const after = source.slice(end).replace(/^\s*/, "");
  return before && after ? `${before}\n\n${after}` : before + after;
}

export function appendEntry(source: string, text: string): string {
  return source.trim() ? `${source.trimEnd()}\n\n${text}\n` : `${text}\n`;
}
