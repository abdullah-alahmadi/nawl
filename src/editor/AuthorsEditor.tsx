// Editor for an "authors" field: one card per author with affiliations, e-mail,
// ORCID, membership, corresponding author, and an optional biography with photo.
import { useState } from "react";
import type { AuthorInfo } from "../bindings/AuthorInfo";
import type { FieldValue } from "../api";
import { pickFigureImage } from "./Figures";

const EMPTY: AuthorInfo = {
  name: "",
  affiliations: [],
  email: "",
  orcid: "",
  membership: "",
  corresponding: false,
  biography: "",
  photo: "",
};

/** Reads structured authors, or converts old "Name | Affiliation | Email" lines. */
export function toAuthors(v: FieldValue | undefined): AuthorInfo[] {
  if (!v) return [];
  if (Array.isArray(v) && v.length > 0 && typeof v[0] === "object") return v as AuthorInfo[];
  const lines = Array.isArray(v) ? (v as string[]) : String(v).split("\n");
  return lines
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l) => {
      const [name = "", affs = "", email = ""] = l.split("|").map((p) => p.trim());
      return { ...EMPTY, name, affiliations: affs.split(";").map((a) => a.trim()).filter(Boolean), email };
    });
}

export function AuthorsEditor({
  value,
  onChange,
  showBiographies,
}: {
  value: FieldValue | undefined;
  onChange: (authors: AuthorInfo[]) => void;
  showBiographies: boolean;
}) {
  const authors = toAuthors(value);
  const [open, setOpen] = useState<number | null>(authors.length === 0 ? null : 0);
  const [problem, setProblem] = useState<string | null>(null);

  const update = (i: number, patch: Partial<AuthorInfo>) => {
    const next = authors.map((a, j) => {
      if (j === i) return { ...a, ...patch };
      // Only one corresponding author.
      return patch.corresponding ? { ...a, corresponding: false } : a;
    });
    onChange(next);
  };
  const move = (i: number, to: number) => {
    if (to < 0 || to >= authors.length) return;
    const next = [...authors];
    const [a] = next.splice(i, 1);
    next.splice(to, 0, a);
    onChange(next);
    setOpen(to);
  };
  const remove = (i: number) => {
    onChange(authors.filter((_, j) => j !== i));
    setOpen(null);
  };
  const add = () => {
    // New authors often share the previous author's affiliation.
    const prev = authors[authors.length - 1];
    onChange([...authors, { ...EMPTY, affiliations: prev ? [...prev.affiliations] : [] }]);
    setOpen(authors.length);
  };
  const choosePhoto = async (i: number) => {
    setProblem(null);
    try {
      const path = await pickFigureImage();
      if (path) update(i, { photo: path });
    } catch (e) {
      setProblem(String((e as { message?: string }).message ?? e));
    }
  };

  return (
    <div className="authors-editor">
      {authors.map((a, i) => (
        <div key={i} className={`author-card${open === i ? " open" : ""}`}>
          <button type="button" className="author-summary" onClick={() => setOpen(open === i ? null : i)} aria-expanded={open === i}>
            <span className="author-n">{i + 1}</span>
            <span className="author-name">{a.name || "(no name)"}</span>
            {a.corresponding && <span className="author-tag">corresponding</span>}
            <span className="muted author-aff">{a.affiliations[0] ?? ""}</span>
          </button>
          {open === i && (
            <div className="author-fields">
              <label>
                Name
                <input value={a.name} onChange={(e) => update(i, { name: e.target.value })} autoFocus={!a.name} />
              </label>
              <label>
                Affiliations <span className="muted">(one per line)</span>
                <textarea
                  rows={2}
                  value={a.affiliations.join("\n")}
                  onChange={(e) => update(i, { affiliations: e.target.value.split("\n") })}
                  onBlur={(e) => update(i, { affiliations: e.target.value.split("\n").map((x) => x.trim()).filter(Boolean) })}
                />
              </label>
              <div className="author-row">
                <label>
                  E-mail
                  <input type="email" value={a.email} onChange={(e) => update(i, { email: e.target.value })} />
                </label>
                <label>
                  ORCID
                  <input value={a.orcid} placeholder="0000-0000-0000-0000" onChange={(e) => update(i, { orcid: e.target.value })} />
                </label>
              </div>
              <div className="author-row">
                <label>
                  Membership <span className="muted">(e.g. Member, IEEE)</span>
                  <input value={a.membership} onChange={(e) => update(i, { membership: e.target.value })} />
                </label>
                <label className="author-check">
                  <input type="checkbox" checked={a.corresponding} onChange={(e) => update(i, { corresponding: e.target.checked })} />
                  Corresponding author
                </label>
              </div>
              {showBiographies && (
                <>
                  <label>
                    Biography <span className="muted">(printed at the end of the paper)</span>
                    <textarea rows={3} value={a.biography} onChange={(e) => update(i, { biography: e.target.value })} />
                  </label>
                  <div className="author-row">
                    <span className="muted">{a.photo ? `Photo: ${a.photo}` : "No photo"}</span>
                    <span>
                      <button type="button" onClick={() => choosePhoto(i)}>
                        {a.photo ? "Change photo…" : "Add photo…"}
                      </button>
                      {a.photo && (
                        <button type="button" onClick={() => update(i, { photo: "" })}>
                          Remove
                        </button>
                      )}
                    </span>
                  </div>
                </>
              )}
              <div className="author-actions">
                <button type="button" disabled={i === 0} onClick={() => move(i, i - 1)}>
                  ↑ Move up
                </button>
                <button type="button" disabled={i === authors.length - 1} onClick={() => move(i, i + 1)}>
                  ↓ Move down
                </button>
                <button type="button" className="danger" onClick={() => remove(i)}>
                  Remove author
                </button>
              </div>
            </div>
          )}
        </div>
      ))}
      {problem && <div className="d-error">{problem}</div>}
      <button type="button" className="d-add" onClick={add}>
        + Add author
      </button>
    </div>
  );
}
