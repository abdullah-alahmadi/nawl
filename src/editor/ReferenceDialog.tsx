// "New reference" / "Edit reference": a form for one bibliography entry, with fields
// for its type, a row per author, and a citation key made from them. It writes
// ordinary BibTeX into the document's bibliography.
import { useEffect, useMemo, useState } from "react";
import { Plus, Trash2, X } from "lucide-react";
import type { BibEntry } from "../api";
import { makeKey, toBibtex, unescapeField, validKey, type Person } from "./bibtex";
import { useDraggable } from "../ui/useDraggable";
import { useT, type MessageKey } from "../i18n";

// Labels are message keys, translated when the form is drawn. A placeholder is either
// a key (words) or a literal example (numbers, BibTeX syntax).
interface FieldDef {
  name: string;
  label: MessageKey;
  required?: boolean;
  placeholder?: string;
  placeholderKey?: MessageKey;
}

const TYPES: { kind: string; label: MessageKey; fields: FieldDef[] }[] = [
  {
    kind: "article",
    label: "editor.refTypeArticle",
    fields: [
      { name: "journal", label: "editor.refFieldJournal", required: true },
      { name: "volume", label: "editor.refFieldVolume" },
      { name: "number", label: "editor.refFieldIssue" },
      { name: "pages", label: "editor.refFieldPages", placeholder: "12–34" },
    ],
  },
  {
    kind: "inproceedings",
    label: "editor.refTypeInproceedings",
    fields: [
      { name: "booktitle", label: "editor.refFieldConference", required: true },
      { name: "pages", label: "editor.refFieldPages" },
      { name: "publisher", label: "editor.refFieldPublisher" },
      { name: "address", label: "editor.refFieldLocation", placeholderKey: "editor.refPhCityCountry" },
    ],
  },
  {
    kind: "book",
    label: "editor.refTypeBook",
    fields: [
      { name: "publisher", label: "editor.refFieldPublisher", required: true },
      { name: "address", label: "editor.refFieldPlace" },
      { name: "edition", label: "editor.refFieldEdition", placeholder: "2nd" },
    ],
  },
  {
    kind: "incollection",
    label: "editor.refTypeIncollection",
    fields: [
      { name: "booktitle", label: "editor.refFieldBooktitle", required: true },
      // BibTeX joins names with "and" whatever the language.
      { name: "editor", label: "editor.refFieldEditors", placeholder: "Jane Doe and John Smith" },
      { name: "publisher", label: "editor.refFieldPublisher" },
      { name: "pages", label: "editor.refFieldPages" },
    ],
  },
  { kind: "phdthesis", label: "editor.refTypePhdthesis", fields: [{ name: "school", label: "editor.refFieldUniversity", required: true }] },
  { kind: "mastersthesis", label: "editor.refTypeMastersthesis", fields: [{ name: "school", label: "editor.refFieldUniversity", required: true }] },
  {
    kind: "techreport",
    label: "editor.refTypeTechreport",
    fields: [
      { name: "institution", label: "editor.refFieldInstitution", required: true },
      { name: "number", label: "editor.refFieldReportNumber" },
    ],
  },
  {
    kind: "misc",
    label: "editor.refTypeMisc",
    fields: [
      { name: "howpublished", label: "editor.refFieldHowpublished", placeholderKey: "editor.refPhHowpublished" },
      { name: "note", label: "editor.refFieldNote", placeholderKey: "editor.refPhNote" },
    ],
  },
];

/** Fields every type has, around the type's own. */
const TITLE: FieldDef = { name: "title", label: "common.title", required: true };
const YEAR: FieldDef = { name: "year", label: "editor.refFieldYear", placeholder: "2026" };
const LINKS: FieldDef[] = [
  { name: "doi", label: "editor.refFieldDoi", placeholder: "10.1000/xyz123" },
  { name: "url", label: "editor.refFieldUrl", placeholder: "https://…" },
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
  const t = useT();
  const known = TYPES.some((ty) => ty.kind === entry?.kind);
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
  const type = TYPES.find((ty) => ty.kind === kind) ?? { kind, fields: [] as FieldDef[] };
  const drag = useDraggable<HTMLDivElement>();

  const autoKey = useMemo(() => makeKey({ kind, key: "", people, fields }, taken), [kind, people, fields, taken]);
  const effectiveKey = keyTouched ? key.trim() : autoKey;

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const shown = [TITLE, YEAR, ...type.fields, ...LINKS];
  const missing = shown.filter((f) => f.required && !(fields[f.name] ?? "").trim()).map((f) => t(f.label));
  const noAuthor = !people.some((p) => p.family.trim()) && kind !== "misc";
  const keyProblem = !effectiveKey ? t("editor.keyNeeded") : !validKey(effectiveKey) ? t("editor.keyInvalid") : taken.has(effectiveKey) ? t("editor.keyTaken") : null;
  const problems = [...missing.map((m) => t("editor.fieldEmpty", { field: m })), ...(noAuthor ? [t("editor.addOneAuthor")] : []), ...(keyProblem ? [keyProblem] : [])];

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
      <div ref={drag.ref} style={drag.style} className="modal ref-dialog" role="dialog" aria-modal="true" aria-label={entry ? t("editor.editReference") : t("editor.newReferenceTitle")}>
        <div className="gallery-head" {...drag.handle} title={t("editor.dragToMove")}>
          <h2>{entry ? t("editor.editReference") : t("editor.newReferenceTitle")}</h2>
          <button className="icon-button" onClick={onClose} aria-label={t("common.close")} title={t("editor.closeEsc")}>
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
            <span>{t("editor.refType")}</span>
            <select value={kind} onChange={(e) => setKind(e.target.value)}>
              {TYPES.map((ty) => (
                <option key={ty.kind} value={ty.kind}>
                  {t(ty.label)}
                </option>
              ))}
              {!known && entry && <option value={entry.kind}>{entry.kind}</option>}
            </select>
          </label>

          <div className="ref-row">
            <span>{t("editor.authors")}</span>
            <div className="ref-people">
              {people.map((p, i) => (
                <div key={i} className="ref-person">
                  <input value={p.given} onChange={(e) => setPerson(i, { given: e.target.value })} placeholder={t("editor.givenNames")} aria-label={t("editor.givenNamesOf", { n: i + 1 })} autoFocus={i === 0 && !entry} />
                  <input value={p.family} onChange={(e) => setPerson(i, { family: e.target.value })} placeholder={p.given.trim() ? t("editor.familyName") : t("editor.familyOrOrg")} aria-label={t("editor.familyNameOf", { n: i + 1 })} />
                  <button type="button" className="icon-button small" onClick={() => setPeople((ps) => (ps.length > 1 ? ps.filter((_, j) => j !== i) : [{ given: "", family: "" }]))} title={t("editor.removeAuthor")} aria-label={t("editor.removeAuthorN", { n: i + 1 })}>
                    <Trash2 size={13} />
                  </button>
                </div>
              ))}
              <button type="button" className="d-add" onClick={() => setPeople((ps) => [...ps, { given: "", family: "" }])}>
                <Plus size={13} aria-hidden="true" /> {t("editor.addAuthor")}
              </button>
              <span className="muted ref-hint">{t("editor.orgHint")}</span>
            </div>
          </div>

          {shown.map((f) => (
            <label key={f.name} className="ref-row">
              <span>
                {t(f.label)}
                {f.required && <span className="required"> *</span>}
              </span>
              {f.name === "title" ? (
                <textarea rows={2} value={fields[f.name] ?? ""} onChange={(e) => setField(f.name, e.target.value)} />
              ) : (
                <input value={fields[f.name] ?? ""} placeholder={f.placeholderKey ? t(f.placeholderKey) : f.placeholder} onChange={(e) => setField(f.name, e.target.value)} />
              )}
            </label>
          ))}

          <label className="ref-row">
            <span>{t("editor.citationKey")}</span>
            <span className="ref-key">
              <input
                className="mono"
                value={effectiveKey}
                readOnly={cited}
                title={cited ? t("editor.keyLockedTitle") : undefined}
                onChange={(e) => {
                  setKeyTouched(true);
                  setKey(e.target.value.replace(/\s/g, ""));
                }}
              />
              <span className="muted ref-hint">
                {cited ? t("editor.keyLockedHint") : keyTouched ? "" : t("editor.keyAutoHint")}
              </span>
            </span>
          </label>

          <div className="ref-foot">
            {onDelete && (
              <button type="button" className="tb-button danger" onClick={onDelete} title={cited ? t("editor.deleteCitedTitle") : undefined}>
                <Trash2 size={14} aria-hidden="true" /> {t("common.delete")}
              </button>
            )}
            <span className="ref-problems muted">{problems[0] ?? ""}</span>
            <button type="button" className="tb-button" onClick={onClose}>
              {t("common.cancel")}
            </button>
            <button type="submit" className="tb-button primary" disabled={problems.length > 0}>
              {entry ? t("common.save") : t("editor.addReference")}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
