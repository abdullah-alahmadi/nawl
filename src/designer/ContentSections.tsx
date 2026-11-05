// Designer panels: metadata fields, front matter, title page summary.
import { useState } from "react";
import type { FieldKind } from "../bindings/FieldKind";
import type { FrontMatterItem } from "../bindings/FrontMatterItem";
import { Row, Select, Toggle, issuesAt } from "./controls";
import { slugId, type SectionProps } from "./common";

const KINDS: { value: FieldKind; label: string }[] = [
  { value: "text", label: "One line" },
  { value: "paragraphs", label: "Paragraphs" },
  { value: "list", label: "List (one per line)" },
];

function move<T>(list: T[], from: number, to: number) {
  if (to < 0 || to >= list.length) return;
  const [item] = list.splice(from, 1);
  list.splice(to, 0, item);
}

function RowButtons({ index, count, onMove, onRemove }: { index: number; count: number; onMove: (to: number) => void; onRemove: () => void }) {
  return (
    <span className="d-row-buttons">
      <button type="button" title="Move up" disabled={index === 0} onClick={() => onMove(index - 1)}>
        ↑
      </button>
      <button type="button" title="Move down" disabled={index === count - 1} onClick={() => onMove(index + 1)}>
        ↓
      </button>
      <button type="button" title="Remove" className="danger" onClick={onRemove}>
        ✕
      </button>
    </span>
  );
}

export function FieldsSection({ t, edit, issues }: SectionProps) {
  return (
    <>
      <p className="d-note">
        Fields are what authors fill in under <strong>Document info</strong>. Use them in the title page and in
        headers and footers as <code>{"{id}"}</code>.
      </p>
      <div className="d-table">
        <div className="d-table-head">
          <span>Label</span>
          <span>ID</span>
          <span>Kind</span>
          <span>Required</span>
          <span />
        </div>
        {t.fields.map((f, i) => (
          <div key={i} className="d-table-row-wrap">
            <div className="d-table-row">
              <input value={f.label} aria-label="Label" onChange={(e) => edit((d) => void (d.fields[i].label = e.target.value))} />
              <input
                className="mono"
                value={f.id}
                aria-label="ID"
                onChange={(e) => edit((d) => void (d.fields[i].id = e.target.value.toLowerCase().replace(/[^a-z0-9_]/g, "_")))}
              />
              <Select value={f.kind} options={KINDS} onChange={(v) => edit((d) => void (d.fields[i].kind = v))} />
              <input type="checkbox" aria-label="Required" checked={f.required} onChange={(e) => edit((d) => void (d.fields[i].required = e.target.checked))} />
              <RowButtons
                index={i}
                count={t.fields.length}
                onMove={(to) => edit((d) => move(d.fields, i, to))}
                onRemove={() => edit((d) => void d.fields.splice(i, 1))}
              />
            </div>
            {issuesAt(issues, `fields[${i}]`).map((iss, n) => (
              <div key={n} className="d-error">
                {iss.message}
              </div>
            ))}
          </div>
        ))}
      </div>
      <button
        type="button"
        className="d-add"
        onClick={() =>
          edit((d) => {
            d.fields.push({ id: slugId("new field", d.fields.map((f) => f.id)), label: "New field", kind: "text", required: false });
          })
        }
      >
        + Add field
      </button>
      {issues.some((i) => i.message.includes("unknown placeholder")) && (
        <p className="d-warning">
          Some text still uses a placeholder whose field was renamed or removed; see the Headers &amp; footers and
          Title page panels.
        </p>
      )}
    </>
  );
}

const FRONT_TYPES: { value: FrontMatterItem["type"]; label: string }[] = [
  { value: "titlePage", label: "Title page" },
  { value: "fieldPage", label: "Text page from a field (abstract…)" },
  { value: "tableOfContents", label: "Table of contents" },
  { value: "listOfFigures", label: "List of figures" },
  { value: "listOfTables", label: "List of tables" },
];

function newItem(type: FrontMatterItem["type"], t: SectionProps["t"]): FrontMatterItem {
  switch (type) {
    case "titlePage":
      return { type };
    case "fieldPage": {
      const field = t.fields.find((f) => f.kind === "paragraphs");
      return { type, title: field?.label ?? "Abstract", field: field?.id ?? "", inToc: true };
    }
    case "tableOfContents":
      return { type, title: "Contents" };
    case "listOfFigures":
      return { type, title: "List of Figures" };
    case "listOfTables":
      return { type, title: "List of Tables" };
  }
}

export function FrontMatterSection({ t, edit, issues }: SectionProps) {
  const [adding, setAdding] = useState<FrontMatterItem["type"]>("fieldPage");
  const paragraphFields = t.fields.filter((f) => f.kind === "paragraphs");
  return (
    <>
      <p className="d-note">Pages before chapter 1, in this order. They use the front-matter page numbers.</p>
      <ol className="d-list">
        {t.frontMatter.map((item, i) => (
          <li key={i}>
            <div className="d-list-row">
              <strong>{FRONT_TYPES.find((f) => f.value === item.type)?.label}</strong>
              <RowButtons
                index={i}
                count={t.frontMatter.length}
                onMove={(to) => edit((d) => move(d.frontMatter, i, to))}
                onRemove={() => edit((d) => void d.frontMatter.splice(i, 1))}
              />
            </div>
            {"title" in item && (
              <Row label="Heading">
                <input
                  value={item.title}
                  onChange={(e) =>
                    edit((d) => {
                      const it = d.frontMatter[i];
                      if ("title" in it) it.title = e.target.value;
                    })
                  }
                />
              </Row>
            )}
            {item.type === "fieldPage" && (
              <>
                <Row label="Shows field" issues={issuesAt(issues, `frontMatter[${i}].field`)}>
                  <Select
                    value={item.field}
                    options={[
                      ...paragraphFields.map((f) => ({ value: f.id, label: f.label })),
                      ...(paragraphFields.some((f) => f.id === item.field) ? [] : [{ value: item.field, label: `${item.field} (missing)` }]),
                    ]}
                    onChange={(v) =>
                      edit((d) => {
                        const it = d.frontMatter[i];
                        if (it.type === "fieldPage") it.field = v;
                      })
                    }
                  />
                </Row>
                <Row label="Contents">
                  <Toggle
                    checked={item.inToc}
                    label="List in the table of contents"
                    onChange={(v) =>
                      edit((d) => {
                        const it = d.frontMatter[i];
                        if (it.type === "fieldPage") it.inToc = v;
                      })
                    }
                  />
                </Row>
              </>
            )}
          </li>
        ))}
      </ol>
      <div className="d-inline">
        <select value={adding} onChange={(e) => setAdding(e.target.value as FrontMatterItem["type"])}>
          {FRONT_TYPES.map((f) => (
            <option key={f.value} value={f.value}>
              {f.label}
            </option>
          ))}
        </select>
        <button type="button" className="d-add" onClick={() => edit((d) => void d.frontMatter.push(newItem(adding, d)))}>
          + Add page
        </button>
      </div>
      {paragraphFields.length === 0 && (
        <p className="d-note">Text pages need a “Paragraphs” field; add one under Fields.</p>
      )}
    </>
  );
}
