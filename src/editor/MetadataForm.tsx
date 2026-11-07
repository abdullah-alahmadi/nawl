// Form for the template's metadata fields (title, team members, abstract…).
// Generated from the template, so every template gets its own form for free.
import type { FieldValue, Metadata, Template } from "../api";
import type { MetadataField } from "../bindings/MetadataField";
import { useState } from "react";
import { AuthorsEditor, toAuthors } from "./AuthorsEditor";
import { pickFigureImage, useDocumentImage } from "./Figures";
import { useT } from "../i18n";
import { formatDate, formatHijri, parseIso } from "./dates";

/** An image field (a logo): a thumbnail, Choose and Remove. The image is copied
 *  into the document's figures/ folder like any figure. */
function ImageField({ value, onChange }: { value: string; onChange: (path: string) => void }) {
  const t = useT();
  const { url } = useDocumentImage(value);
  const [problem, setProblem] = useState<string | null>(null);
  const choose = async () => {
    setProblem(null);
    try {
      const path = await pickFigureImage();
      if (path) onChange(path);
    } catch (e) {
      setProblem(String((e as { message?: string }).message ?? e));
    }
  };
  return (
    <span className="image-field">
      <span className="image-field-thumb">{url ? <img src={url} alt="" /> : <span className="muted">{value ? value : t("editor.imageFieldEmpty")}</span>}</span>
      <span className="image-field-actions">
        <button type="button" onClick={choose}>
          {value ? t("editor.changeImage") : t("editor.chooseImage")}
        </button>
        {value && (
          <button type="button" onClick={() => onChange("")}>
            {t("common.remove")}
          </button>
        )}
      </span>
      {problem && <span className="d-error">{problem}</span>}
    </span>
  );
}

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
  const t = useT();
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
      <p className="muted metadata-help">{t("editor.metadataHelp", { name: template.meta.name })}</p>
      {template.fields.map((f) => {
        const id = `field-${f.id}`;
        if (f.kind === "authors") {
          const authors = toAuthors(metadata[f.id]);
          const missing = f.required && authors.every((a) => !a.name.trim());
          return (
            <div key={f.id} className={`field-group${missing ? " missing" : ""}`}>
              <span className="field-label">
                {f.label}
                {f.required && <span className="required" title={t("editor.required")}> *</span>}
              </span>
              <AuthorsEditor
                value={metadata[f.id]}
                showBiographies={!!template.base?.biographies}
                simple={template.kind === "presentation"}
                noun={template.kind === "presentation" ? "presenter" : "author"}
                bilingual={template.language?.main === "arabic"}
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
        if (f.kind === "image")
          return (
            <div key={f.id} className={`field-group${missing ? " missing" : ""}`}>
              <span className="field-label">
                {f.label}
                {f.required && <span className="required" title={t("editor.required")}> *</span>}
              </span>
              <ImageField value={value} onChange={(path) => set(f, path)} />
            </div>
          );
        return (
          <label key={f.id} htmlFor={id} className={missing ? "missing" : ""}>
            <span>
              {f.label}
              {f.required && <span className="required" title={t("editor.required")}> *</span>}
              {f.kind === "list" && <span className="muted"> {t("editor.onePerLine")}</span>}
            </span>
            {f.kind === "date" && (value === "" || parseIso(value)) ? (
              <>
                <input id={id} type="date" value={value} onChange={(e) => set(f, e.target.value)} />
                {value && (
                  // How the document writes it, and the Hijri date for {field_hijri}.
                  <span className="muted metadata-date">
                    {formatDate(value, template.language?.main ?? "english", template.language?.digits ?? "western")}
                    {" · "}
                    {formatHijri(value, template.language?.main ?? "english", template.language?.digits ?? "western")}
                  </span>
                )}
              </>
            ) : f.kind === "text" || f.kind === "date" ? (
              // A date typed as text before the field became a date stays editable as text.
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
