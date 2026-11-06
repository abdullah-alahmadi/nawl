// "New reference" / "Edit reference": a form for one bibliography entry, with fields
// for its type, a row per author, and a citation key made from them. It writes
// ordinary BibTeX into the document's bibliography.
import { useEffect, useMemo, useState } from "react";
import { Plus, Trash2, X } from "lucide-react";
import type { BibEntry } from "../api";
import { makeKey, toBibtex, unescapeField, validKey, type Person } from "./bibtex";

interface FieldDef {
  name: string;
  label: string;
  required?: boolean;
  placeholder?: string;
}

const TYPES: { kind: string; label: string; fields: FieldDef[] }[] = [
  {
    kind: "article",
    label: "Journal article",
    fields: [
      { name: "journal", label: "Journal", required: true },
      { name: "volume", label: "Volume" },
      { name: "number", label: "Issue" },
      { name: "pages", label: "Pages", placeholder: "12–34" },
    ],
  },
  {
    kind: "inproceedings",
    label: "Conference paper",
    fields: [
      { name: "booktitle", label: "Conference or proceedings", required: true },
      { name: "pages", label: "Pages" },
      { name: "publisher", label: "Publisher" },
      { name: "address", label: "Location", placeholder: "City, Country" },
    ],
  },
  {
    kind: "book",
    label: "Book",
    fields: [
      { name: "publisher", label: "Publisher", required: true },
      { name: "address", label: "Place" },
      { name: "edition", label: "Edition", placeholder: "2nd" },
    ],
  },
  {
    kind: "incollection",
    label: "Book chapter",
    fields: [
      { name: "booktitle", label: "Book title", required: true },
      { name: "editor", label: "Editors", placeholder: "Jane Doe and John Smith" },
      { name: "publisher", label: "Publisher" },
      { name: "pages", label: "Pages" },
    ],
  },
  { kind: "phdthesis", label: "PhD thesis", fields: [{ name: "school", label: "University", required: true }] },
  { kind: "mastersthesis", label: "Master's thesis", fields: [{ name: "school", label: "University", required: true }] },
  {
    kind: "techreport",
    label: "Report",
    fields: [
      { name: "institution", label: "Institution", required: true },
      { name: "number", label: "Report number" },
    ],
  },
  {
    kind: "misc",
    label: "Website or other",
    fields: [
      { name: "howpublished", label: "Published as", placeholder: "Website, dataset, software…" },
      { name: "note", label: "Note", placeholder: "Accessed 5 May 2026" },
    ],
  },
];

/** Fields every type has, around the type's own. */
const TITLE: FieldDef = { name: "title", label: "Title", required: true };
const YEAR: FieldDef = { name: "year", label: "Year", placeholder: "2026" };
const LINKS: FieldDef[] = [
  { name: "doi", label: "DOI", placeholder: "10.1000/xyz123" },
  { name: "url", label: "URL", placeholder: "https://…" },
];

export function ReferenceDialog({
  entry,
  taken,
  cited,
  onSave,
  onDelete,
  onClose,
}: {
  /** The entry being edited; absent for a new one. */
  entry?: BibEntry;
  /** Keys already in the bibliography (other than this entry's). */
  taken: Set<string>;
  /** The entry is cited in the text (its key can't change then). */
  cited: boolean;
  onSave: (bibtex: string, key: string) => void;
  onDelete?: () => void;
  onClose: () => void;
}) {
  const known = TYPES.some((t) => t.kind === entry?.kind);
  const [kind, setKind] = useState(entry?.kind ?? "article");
  const [people, setPeople] = useState<Person[]>(entry?.people.length ? entry.people.map((p) => ({ ...p })) : [{ given: "", family: "" }]);
  // Every field of the entry is kept, also ones the form doesn't show.
  const [fields, setFields] = useState<Record<string, string>>(() => {
    const out: Record<string, string> = {};
    for (const [k, v] of Object.entries(entry?.fields ?? {})) if (k !== "author") out[k] = unescapeField(v);
    return out;
  });
  const [key, setKey] = useState(entry?.key ?? "");
  // A new entry's key follows the authors, year and title until it's typed in.
  const [keyTouched, setKeyTouched] = useState(!!entry);
  const type = TYPES.find((t) => t.kind === kind) ?? { kind, label: kind, fields: [] };

  const autoKey = useMemo(() => makeKey({ kind, key: "", people, fields }, taken), [kind, people, fields, taken]);
  const effectiveKey = keyTouched ? key.trim() : autoKey;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const shown = [TITLE, YEAR, ...type.fields, ...LINKS];
  const missing = shown.filter((f) => f.required && !(fields[f.name] ?? "").trim()).map((f) => f.label);
  const noAuthor = !people.some((p) => p.family.trim()) && kind !== "misc";
  const keyProblem = !effectiveKey ? "Needs a citation key" : !validKey(effectiveKey) ? "Keys may use letters, digits and - _ : . / +" : taken.has(effectiveKey) ? "Another reference has this key" : null;
  const problems = [...missing.map((m) => `${m} is empty`), ...(noAuthor ? ["Add at least one author"] : []), ...(keyProblem ? [keyProblem] : [])];

  const setField = (name: string, value: string) => setFields((f) => ({ ...f, [name]: value }));
  const setPerson = (i: number, patch: Partial<Person>) => setPeople((ps) => ps.map((p, j) => (j === i ? { ...p, ...patch } : p)));

  const save = () => {
    if (problems.length) return;
    // Fields of other types aren't carried over when the type changes.
    const keep = new Set([...shown.map((f) => f.name), ...Object.keys(entry?.fields ?? {})]);
    const out: Record<string, string> = {};
    for (const f of shown) if (fields[f.name]?.trim()) out[f.name] = fields[f.name];
    for (const [k, v] of Object.entries(fields)) if (!(k in out) && keep.has(k) && v.trim() && !shown.some((f) => f.name === k)) out[k] = v;
    onSave(toBibtex({ kind, key: effectiveKey, people, fields: out }), effectiveKey);
  };

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal ref-dialog" role="dialog" aria-modal="true" aria-label={entry ? "Edit reference" : "New reference"}>
        <div className="gallery-head">
          <h2>{entry ? "Edit reference" : "New reference"}</h2>
          <button className="icon-button" onClick={onClose} aria-label="Close" title="Close (Esc)">
            <X size={17} />
          </button>
        </div>
        <form
          className="ref-form"
          onSubmit={(e) => {
            e.preventDefault();
            save();
          }}
        >
          <label className="ref-row">
            <span>Type</span>
            <select value={kind} onChange={(e) => setKind(e.target.value)}>
              {TYPES.map((t) => (
                <option key={t.kind} value={t.kind}>
                  {t.label}
                </option>
              ))}
              {!known && entry && <option value={entry.kind}>{entry.kind}</option>}
            </select>
          </label>

          <div className="ref-row">
            <span>Authors</span>
            <div className="ref-people">
              {people.map((p, i) => (
                <div key={i} className="ref-person">
                  <input value={p.given} onChange={(e) => setPerson(i, { given: e.target.value })} placeholder="Given names" aria-label={`Author ${i + 1} given names`} autoFocus={i === 0 && !entry} />
                  <input value={p.family} onChange={(e) => setPerson(i, { family: e.target.value })} placeholder={p.given.trim() ? "Family name" : "Family name or organisation"} aria-label={`Author ${i + 1} family name`} />
                  <button type="button" className="icon-button small" onClick={() => setPeople((ps) => (ps.length > 1 ? ps.filter((_, j) => j !== i) : [{ given: "", family: "" }]))} title="Remove author" aria-label={`Remove author ${i + 1}`}>
                    <Trash2 size={13} />
                  </button>
                </div>
              ))}
              <button type="button" className="d-add" onClick={() => setPeople((ps) => [...ps, { given: "", family: "" }])}>
                <Plus size={13} aria-hidden="true" /> Add author
              </button>
              <span className="muted ref-hint">For an organisation, leave the given names empty.</span>
            </div>
          </div>

          {shown.map((f) => (
            <label key={f.name} className="ref-row">
              <span>
                {f.label}
                {f.required && <span className="required"> *</span>}
              </span>
              {f.name === "title" ? (
                <textarea rows={2} value={fields[f.name] ?? ""} onChange={(e) => setField(f.name, e.target.value)} />
              ) : (
                <input value={fields[f.name] ?? ""} placeholder={f.placeholder} onChange={(e) => setField(f.name, e.target.value)} />
              )}
            </label>
          ))}

          <label className="ref-row">
            <span>Citation key</span>
            <span className="ref-key">
              <input
                className="mono"
                value={effectiveKey}
                readOnly={cited}
                title={cited ? "Cited in the text: changing the key would break those citations" : undefined}
                onChange={(e) => {
                  setKeyTouched(true);
                  setKey(e.target.value.replace(/\s/g, ""));
                }}
              />
              <span className="muted ref-hint">
                {cited ? "Cited in the text, so the key stays." : keyTouched ? "" : "Made from the first author, year and title."}
              </span>
            </span>
          </label>

          <div className="ref-foot">
            {onDelete && (
              <button type="button" className="tb-button danger" onClick={onDelete} title={cited ? "Its citations will show as missing" : undefined}>
                <Trash2 size={14} aria-hidden="true" /> Delete
              </button>
            )}
            <span className="ref-problems muted">{problems[0] ?? ""}</span>
            <button type="button" className="tb-button" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" className="tb-button primary" disabled={problems.length > 0}>
              {entry ? "Save" : "Add reference"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
