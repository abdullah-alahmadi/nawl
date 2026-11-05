// Designer panels: metadata fields, front matter, title page summary.
import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { importKitZip, importTemplateFile } from "../api";
import type { FieldKind } from "../bindings/FieldKind";
import type { FrontMatterItem } from "../bindings/FrontMatterItem";
import { Row, Select, Toggle, issuesAt } from "./controls";
import { slugId, type SectionProps } from "./common";

const KINDS: { value: FieldKind; label: string }[] = [
  { value: "text", label: "One line" },
  { value: "paragraphs", label: "Paragraphs" },
  { value: "list", label: "List (one per line)" },
  { value: "authors", label: "Authors (affiliations, e-mail…)" },
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

const BIB_STYLES = [
  { value: "ieee", label: "IEEE — [1], numbered in order of citation" },
  { value: "numeric", label: "Numeric — [1], list sorted by author" },
  { value: "authorYear", label: "Author–year — (Smith, 2021), APA-like" },
] as const;

export function BibliographySection({ t, edit, issues }: SectionProps) {
  const b = t.bibliography;
  return (
    <>
      <Row label="Citation style">
        <Select value={b.style} options={[...BIB_STYLES]} onChange={(v) => edit((d) => void (d.bibliography.style = v))} />
      </Row>
      <Row label="List heading" issues={issuesAt(issues, "bibliography.title")}>
        <input value={b.title} onChange={(e) => edit((d) => void (d.bibliography.title = e.target.value))} />
      </Row>
      <Row label="Contents">
        <Toggle checked={b.inToc} label="List it in the table of contents" onChange={(v) => edit((d) => void (d.bibliography.inToc = v))} />
      </Row>
      <p className="d-note">
        The reference list appears after the last chapter (before any appendices) when the document cites something.
        References are formatted by BibTeX, so they follow the chosen style exactly.
      </p>
    </>
  );
}

export function SkeletonSection({ t, edit, issues }: SectionProps) {
  const level = t.structure.topLevel === "chapter" ? "chapter" : "section";
  return (
    <>
      <p className="d-note">
        A new document starts with these {level}s. Required ones are checked: authors see a warning if one is missing.
      </p>
      <div className="d-table">
        <div className="d-table-head skeleton">
          <span>Title</span>
          <span>Required</span>
          <span />
        </div>
        {t.skeleton.map((sec, i) => (
          <div key={i} className="d-table-row-wrap">
            <div className="d-table-row skeleton">
              <input value={sec.title} aria-label="Title" onChange={(e) => edit((d) => void (d.skeleton[i].title = e.target.value))} />
              <input type="checkbox" aria-label="Required" checked={sec.required} onChange={(e) => edit((d) => void (d.skeleton[i].required = e.target.checked))} />
              <RowButtons
                index={i}
                count={t.skeleton.length}
                onMove={(to) => edit((d) => move(d.skeleton, i, to))}
                onRemove={() => edit((d) => void d.skeleton.splice(i, 1))}
              />
            </div>
            {issuesAt(issues, `skeleton[${i}]`).map((iss, n) => (
              <div key={n} className="d-error">
                {iss.message}
              </div>
            ))}
          </div>
        ))}
      </div>
      <button type="button" className="d-add" onClick={() => edit((d) => void d.skeleton.push({ title: `New ${level}`, required: false }))}>
        + Add {level}
      </button>
    </>
  );
}

const FRONT_STYLES = [
  { value: "ieeeJournal", label: "IEEE journal (affiliations in a footnote)" },
  { value: "ieeeConference", label: "IEEE conference (author blocks)" },
  { value: "elsevier", label: "Elsevier (frontmatter, numbered affiliations)" },
  { value: "acm", label: "ACM (per-author affiliation)" },
  { value: "lncs", label: "Springer LNCS (institutes)" },
  { value: "revtex", label: "APS REVTeX (per-author affiliation)" },
  { value: "ieeeAccess", label: "IEEE Access (\\address, \\corresp, \\EOD)" },
  { value: "custom", label: "Custom recipe (write it below)" },
] as const;

/** A starting point for custom recipes, showing every construct. */
const EXAMPLE_RECIPE = String.raw`\title{{title}}
\author{{#authors| \and }{name}\textsuperscript{{affnums}}{?corresponding}\thanks{Corresponding author: {email}}{/corresponding}{/authors}}
\date{}
\maketitle
\begin{center}\small
{#affiliations|\\}\textsuperscript{{n}}{name}{/affiliations}
\end{center}
\begin{abstract}
{abstract}
\end{abstract}
{?keywords}\noindent\textbf{Keywords:} {keywords}{/keywords}`;

/** Settings for templates built on a publisher's official class. */
export function PublisherSection({
  t,
  edit,
  issues,
  canImport,
}: SectionProps & { canImport: boolean }) {
  const base = t.base;
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  if (!base) return null;
  const kit = base.kit;
  const haveNames = base.files.map((f) => f.split("/").pop()!);
  const kitMissing = kit ? kit.required.filter((r) => !haveNames.includes(r)) : [];
  const importZip = async () => {
    setProblem(null);
    const file = await open({ multiple: false, directory: false, filters: [{ name: "Publisher LaTeX kit", extensions: ["zip"] }] });
    if (!file) return;
    setBusy(true);
    try {
      const added = await importKitZip(file);
      edit((d) => {
        if (!d.base) return;
        for (const a of added) if (!d.base.files.includes(a)) d.base.files.push(a);
        // Use the kit's class: the one the template expects, else the only one.
        const classes = added.filter((a) => a.endsWith(".cls")).map((a) => a.split("/").pop()!.replace(/\.cls$/, ""));
        const expected = d.base.kit?.required.find((r) => r.endsWith(".cls"))?.replace(/\.cls$/, "");
        if (expected && classes.includes(expected)) d.base.class = expected;
        else if (!expected && classes.length === 1) d.base.class = classes[0];
      });
    } catch (e) {
      setProblem(String((e as { message?: string }).message ?? e));
    } finally {
      setBusy(false);
    }
  };
  const importFiles = async () => {
    setProblem(null);
    const picked = await open({
      multiple: true,
      directory: false,
      filters: [{ name: "LaTeX kit files", extensions: ["cls", "sty", "bst", "clo", "cfg", "def", "fd", "png", "jpg", "jpeg", "pdf", "eps", "otf", "ttf"] }],
    });
    const files = Array.isArray(picked) ? picked : picked ? [picked] : [];
    const added: string[] = [];
    for (const f of files) {
      try {
        added.push(await importTemplateFile(f));
      } catch (e) {
        setProblem(String((e as { message?: string }).message ?? e));
      }
    }
    if (added.length === 0) return;
    edit((d) => {
      if (!d.base) return;
      for (const a of added) if (!d.base.files.includes(a)) d.base.files.push(a);
      // Importing a single class: use it.
      const cls = added.find((a) => a.endsWith(".cls"));
      if (cls) d.base.class = cls.split("/").pop()!.replace(/\.cls$/, "");
    });
  };
  return (
    <>
      {kit && (
        <div className={`kit-card${kitMissing.length ? " missing" : ""}`}>
          <strong>{kitMissing.length ? `This template needs the ${kit.name}` : `${kit.name}: imported`}</strong>
          {kitMissing.length > 0 && (
            <>
              <p>
                The publisher distributes it separately, so it can’t come with Texloom. It takes a minute, once:
              </p>
              <ol>
                <li>
                  <button type="button" className="link" onClick={() => openUrl(kit.url)}>
                    Download the kit
                  </button>{" "}
                  (a .zip) from the publisher.
                </li>
                <li>{canImport ? "Import the .zip:" : "Save this template (New copy → Save as…), then import the .zip:"}</li>
              </ol>
              <button type="button" className="primary" onClick={importZip} disabled={!canImport || busy}>
                {busy ? "Importing…" : "Import kit (.zip)…"}
              </button>
            </>
          )}
        </div>
      )}
      <p className="d-note">
        This template uses the publisher’s official LaTeX class, so page layout, fonts, headings and captions follow
        the publisher exactly and can’t be changed here.
      </p>
      <Row label="Class" issues={issuesAt(issues, "base.class")}>
        <input className="mono" value={base.class} onChange={(e) => edit((d) => void (d.base && (d.base.class = e.target.value.trim())))} />
      </Row>
      <Row label="Class options" hint="Comma-separated" issues={issuesAt(issues, "base.options")}>
        <input
          className="mono"
          value={base.options.join(", ")}
          onChange={(e) =>
            edit((d) => {
              if (d.base) d.base.options = e.target.value.split(",").map((o) => o.trim()).filter(Boolean);
            })
          }
        />
      </Row>
      <Row label="Title block style">
        <Select value={base.frontMatter} options={[...FRONT_STYLES]} onChange={(v) => edit((d) => void (d.base && (d.base.frontMatter = v)))} />
      </Row>
      {base.frontMatter === "custom" && (
        <>
          <Row label="Title block recipe" hint="LaTeX with placeholders" issues={issuesAt(issues, "base.titleBlock")}>
            <div className="d-control">
              <textarea
                className="mono recipe"
                rows={10}
                spellCheck={false}
                value={base.titleBlock ?? ""}
                onChange={(e) => edit((d) => void (d.base && (d.base.titleBlock = e.target.value)))}
              />
              {!base.titleBlock && (
                <button type="button" className="d-add" onClick={() => edit((d) => void (d.base && (d.base.titleBlock = EXAMPLE_RECIPE)))}>
                  Start from an example
                </button>
              )}
              <details className="recipe-help">
                <summary>Placeholders</summary>
                <p>
                  <code>{"{title}"}</code> <code>{"{abstract}"}</code> <code>{"{keywords}"}</code>, any field by id,{" "}
                  <code>{"{corresponding}"}</code>, <code>{"{corresponding_email}"}</code>.
                </p>
                <p>
                  <code>{"{#authors| \\and }…{/authors}"}</code> repeats per author (text after <code>|</code> goes
                  between them): <code>{"{name}"}</code> <code>{"{email}"}</code> <code>{"{orcid}"}</code>{" "}
                  <code>{"{membership}"}</code> <code>{"{affiliation}"}</code> <code>{"{affiliations}"}</code>{" "}
                  <code>{"{affnums}"}</code> <code>{"{n}"}</code>.
                </p>
                <p>
                  <code>{"{#affiliations|, }…{/affiliations}"}</code>: <code>{"{n}"}</code> <code>{"{name}"}</code>{" "}
                  <code>{"{emails}"}</code>. <code>{"{#keywords| \\sep }{keyword}{/keywords}"}</code>.
                </p>
                <p>
                  <code>{"{?email}…{/email}"}</code> includes the part only when the value isn’t empty. Everything else
                  is written as-is; values are escaped for LaTeX.
                </p>
              </details>
            </div>
          </Row>
          <Row label="End block" hint="Optional, after everything" issues={issuesAt(issues, "base.endBlock")}>
            <textarea
              className="mono recipe"
              rows={4}
              spellCheck={false}
              value={base.endBlock ?? ""}
              onChange={(e) =>
                edit((d) => {
                  if (!d.base) return;
                  if (e.target.value) d.base.endBlock = e.target.value;
                  else delete d.base.endBlock;
                })
              }
            />
          </Row>
        </>
      )}
      <Row label="Biographies">
        <Toggle
          checked={base.biographies}
          label="Print author biographies (with photos) at the end"
          onChange={(v) => edit((d) => void (d.base && (d.base.biographies = v)))}
        />
      </Row>
      <Row label="BibTeX style" issues={issuesAt(issues, "base.bibStyle")}>
        <input className="mono" value={base.bibStyle} onChange={(e) => edit((d) => void (d.base && (d.base.bibStyle = e.target.value.trim())))} />
      </Row>
      <Row label="Publisher files" hint="From the publisher's LaTeX kit" issues={issuesAt(issues, "base.files")}>
        <div className="d-control">
          {base.files.length === 0 && <span className="muted">None: the class comes with the TeX engine.</span>}
          {base.files.map((f, i) => (
            <span key={f} className="d-inline">
              <code>{f}</code>
              <button type="button" className="danger" onClick={() => edit((d) => void d.base?.files.splice(i, 1))}>
                Remove
              </button>
            </span>
          ))}
          <span className="d-inline">
            <button type="button" className="d-add" onClick={importZip} disabled={!canImport || busy} title={canImport ? "" : "Save the template first"}>
              + Import kit (.zip)…
            </button>
            <button type="button" className="d-add" onClick={importFiles} disabled={!canImport} title={canImport ? "" : "Save the template first"}>
              + Import single files…
            </button>
          </span>
          {!canImport && <span className="d-hint">Save the template first (Save as…) so the files have a folder.</span>}
          {problem && <span className="d-error">{problem}</span>}
        </div>
      </Row>
      <p className="d-note">
        <strong>Other journals</strong> (Springer Nature, MDPI, Taylor &amp; Francis…): download the publisher’s LaTeX
        kit, start from the closest built-in template, choose <em>New copy</em> and <em>Save as…</em>, import the kit
        here, and pick the title block style — or write a <em>Custom recipe</em> if none fits.
      </p>
    </>
  );
}
