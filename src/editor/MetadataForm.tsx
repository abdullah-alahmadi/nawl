// Form for the template's metadata fields (title, team members, abstract…).
// Generated from the template, so every template gets its own form for free.
import type { FieldValue, Metadata, Template } from "../api";
import type { MetadataField } from "../bindings/MetadataField";
import { AuthorsEditor, toAuthors } from "./AuthorsEditor";

function asText(v: FieldValue | undefined): string {
  if (v === undefined) return "";
  if (Array.isArray(v)) return v.map((x) => (typeof x === "string" ? x : x.name)).join("\n");
  return v;
}

export function MetadataForm({
  template,
  metadata,
  onChange,
}: {
  template: Template;
  metadata: Metadata;
  onChange: (m: Metadata) => void;
}) {
  const set = (field: MetadataField, raw: string) => {
    const next = { ...metadata };
    // List fields store one entry per line; empty values are removed.
    const value: FieldValue = field.kind === "list" ? raw.split("\n") : raw;
    if (raw.trim() === "") delete next[field.id];
    else next[field.id] = value;
    onChange(next);
  };

  return (
    <form className="metadata-form" onSubmit={(e) => e.preventDefault()}>
      <h2>Document info</h2>
      <p className="muted">Fields defined by the “{template.meta.name}” template.</p>
      {template.fields.map((f) => {
        const id = `field-${f.id}`;
        if (f.kind === "authors") {
          const authors = toAuthors(metadata[f.id]);
          const missing = f.required && authors.every((a) => !a.name.trim());
          return (
            <div key={f.id} className={`field-group${missing ? " missing" : ""}`}>
              <span className="field-label">
                {f.label}
                {f.required && <span className="required" title="Required"> *</span>}
              </span>
              <AuthorsEditor
                value={metadata[f.id]}
                showBiographies={!!template.base?.biographies}
                onChange={(list) => {
                  const next = { ...metadata };
                  if (list.length === 0) delete next[f.id];
                  else next[f.id] = list;
                  onChange(next);
                }}
              />
            </div>
          );
        }
        const value = asText(metadata[f.id]);
        const missing = f.required && value.trim() === "";
        return (
          <label key={f.id} htmlFor={id} className={missing ? "missing" : ""}>
            <span>
              {f.label}
              {f.required && <span className="required" title="Required"> *</span>}
              {f.kind === "list" && <span className="muted"> (one per line)</span>}
            </span>
            {f.kind === "text" ? (
              <input id={id} value={value} onChange={(e) => set(f, e.target.value)} />
            ) : (
              <textarea
                id={id}
                rows={f.kind === "paragraphs" ? 6 : 4}
                value={value}
                onChange={(e) => set(f, e.target.value)}
              />
            )}
          </label>
        );
      })}
    </form>
  );
}
