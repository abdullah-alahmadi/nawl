// Designer panels: metadata fields, front matter, title page summary.
import { Fragment, useState, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import { importTemplateFile, type Template } from "../api";
import type { BibStyle } from "../bindings/BibStyle";
import type { FieldKind } from "../bindings/FieldKind";
import type { FrontMatterStyle } from "../bindings/FrontMatterStyle";
import type { FrontMatterItem } from "../bindings/FrontMatterItem";
import { Row, Select, Toggle, issuesAt } from "./controls";
import { fieldUses, slugId, type SectionProps } from "./common";
import { approvalPageItems } from "./TitlePageCanvas";
import { useT, type MessageKey } from "../i18n";

type Translate = ReturnType<typeof useT>;

// Option lists hold message keys: they're made at import, before the interface
// language is known, and translated when shown.
function options<T>(list: readonly { value: T; label: MessageKey }[], tr: Translate): { value: T; label: string }[] {
  return list.map((o) => ({ value: o.value, label: tr(o.label) }));
}

/** A translated sentence with elements (<code>, <strong>…) in place of its {name} parts. */
function rich(text: string, parts: Record<string, ReactNode>): ReactNode[] {
  return text.split(/(\{[a-zA-Z]+\})/).map((piece, i) => {
    const name = /^\{([a-zA-Z]+)\}$/.exec(piece)?.[1];
    return name !== undefined && name in parts ? <Fragment key={i}>{parts[name]}</Fragment> : piece;
  });
}

const KINDS: { value: FieldKind; label: MessageKey }[] = [
  { value: "text", label: "designerContent.fieldKindText" },
  { value: "paragraphs", label: "designerContent.fieldKindParagraphs" },
  { value: "list", label: "designerContent.fieldKindList" },
  { value: "image", label: "designerContent.fieldKindImage" },
  { value: "authors", label: "designerContent.fieldKindAuthors" },
  { value: "date", label: "designerContent.fieldKindDate" },
];

function move<T>(list: T[], from: number, to: number) {
  if (to < 0 || to >= list.length) return;
  const [item] = list.splice(from, 1);
  list.splice(to, 0, item);
}

function RowButtons({ index, count, onMove, onRemove }: { index: number; count: number; onMove: (to: number) => void; onRemove: () => void }) {
  const tr = useT();
  return (
    <span className="d-row-buttons">
      <button type="button" title={tr("common.moveUp")} disabled={index === 0} onClick={() => onMove(index - 1)}>
        ↑
      </button>
      <button type="button" title={tr("common.moveDown")} disabled={index === count - 1} onClick={() => onMove(index + 1)}>
        ↓
      </button>
      <button type="button" title={tr("common.remove")} className="danger" onClick={onRemove}>
        ✕
      </button>
    </span>
  );
}

export function FieldsSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const uses = fieldUses(t);
  const kinds = options(KINDS, tr);
  return (
    <>
      <p className="d-note">
        {rich(tr("designerContent.fieldsNote"), {
          documentInfo: <strong>{tr("designerContent.documentInfo")}</strong>,
          placeholder: <code>{"{id}"}</code>,
        })}
      </p>
      <div className="d-table">
        <div className="d-table-head">
          <span>{tr("designerContent.colLabel")}</span>
          <span>{tr("designerContent.colId")}</span>
          <span>{tr("designerContent.colKind")}</span>
          <span>{tr("designerContent.colRequired")}</span>
          <span />
        </div>
        {t.fields.map((f, i) => (
          <div key={i} className="d-table-row-wrap">
            <div className="d-table-row">
              <input value={f.label} aria-label={tr("designerContent.colLabel")} onChange={(e) => edit((d) => void (d.fields[i].label = e.target.value))} />
              <input
                className="mono"
                value={f.id}
                aria-label={tr("designerContent.colId")}
                onChange={(e) => edit((d) => void (d.fields[i].id = e.target.value.toLowerCase().replace(/[^a-z0-9_]/g, "_")))}
              />
              <Select value={f.kind} options={kinds} onChange={(v) => edit((d) => void (d.fields[i].kind = v))} />
              <input type="checkbox" aria-label={tr("designerContent.colRequired")} checked={f.required} onChange={(e) => edit((d) => void (d.fields[i].required = e.target.checked))} />
              <RowButtons
                index={i}
                count={t.fields.length}
                onMove={(to) => edit((d) => move(d.fields, i, to))}
                onRemove={() => edit((d) => void d.fields.splice(i, 1))}
              />
            </div>
            {uses.has(f.id) ? (
              <div className="d-hint">{tr("designerContent.shownIn", { places: uses.get(f.id)!.join(tr("designerContent.and")) })}</div>
            ) : (
              <div className="d-hint">{rich(tr("designerContent.notShown"), { placeholder: <code>{`{${f.id}}`}</code> })}</div>
            )}
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
        {tr("designerContent.addField")}
      </button>
      {issues.some((i) => i.message.includes("unknown placeholder")) && <p className="d-warning">{tr("designerContent.unknownPlaceholderWarning")}</p>}
    </>
  );
}

const FRONT_TYPES: { value: FrontMatterItem["type"]; label: MessageKey }[] = [
  { value: "titlePage", label: "designerContent.titlePage" },
  { value: "fieldPage", label: "designerContent.frontFieldPage" },
  { value: "tableOfContents", label: "designerContent.frontToc" },
  { value: "listOfFigures", label: "designerContent.frontLof" },
  { value: "listOfTables", label: "designerContent.frontLot" },
  { value: "fixedPage", label: "designerContent.frontFixedPage" },
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
    case "fixedPage": {
      const p = t.page.paper;
      const width = p.kind === "custom" ? p.widthMm : p.kind === "a5" ? 148 : p.kind === "a4" ? 210 : 215.9;
      return { type, title: "Approval page", items: approvalPageItems(width), inToc: false };
    }
  }
}

export function FrontMatterSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const [adding, setAdding] = useState<FrontMatterItem["type"]>("fieldPage");
  const paragraphFields = t.fields.filter((f) => f.kind === "paragraphs");
  return (
    <>
      <p className="d-note">{tr("designerContent.frontMatterNote")}</p>
      <ol className="d-list">
        {t.frontMatter.map((item, i) => (
          <li key={i}>
            <div className="d-list-row">
              <strong>{tr(FRONT_TYPES.find((f) => f.value === item.type)?.label ?? "designerContent.fixedPage")}</strong>
              <RowButtons
                index={i}
                count={t.frontMatter.length}
                onMove={(to) => edit((d) => move(d.frontMatter, i, to))}
                onRemove={() => edit((d) => void d.frontMatter.splice(i, 1))}
              />
            </div>
            {"title" in item && (
              <Row label={tr("designerContent.heading")}>
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
            {item.type === "fixedPage" && (
              <>
                <Row label={tr("designerContent.contents")}>
                  <Toggle
                    checked={item.inToc}
                    label={tr("designerContent.listInToc")}
                    onChange={(v) =>
                      edit((d) => {
                        const it = d.frontMatter[i];
                        if (it.type === "fixedPage") it.inToc = v;
                      })
                    }
                  />
                </Row>
                <p className="d-note">{tr("designerContent.fixedPageLayoutNote")}</p>
              </>
            )}
            {item.type === "fieldPage" && (
              <>
                <Row label={tr("designerContent.showsField")} issues={issuesAt(issues, `frontMatter[${i}].field`)}>
                  <Select
                    value={item.field}
                    options={[
                      ...paragraphFields.map((f) => ({ value: f.id, label: f.label })),
                      ...(paragraphFields.some((f) => f.id === item.field) ? [] : [{ value: item.field, label: tr("designerContent.missingField", { field: item.field }) }]),
                    ]}
                    onChange={(v) =>
                      edit((d) => {
                        const it = d.frontMatter[i];
                        if (it.type === "fieldPage") it.field = v;
                      })
                    }
                  />
                </Row>
                <Row label={tr("designerContent.contents")}>
                  <Toggle
                    checked={item.inToc}
                    label={tr("designerContent.listInToc")}
                    onChange={(v) =>
                      edit((d) => {
                        const it = d.frontMatter[i];
                        if (it.type === "fieldPage") it.inToc = v;
                      })
                    }
                  />
                </Row>
                <Row label={tr("designerContent.placement")}>
                  <Toggle
                    checked={!!item.inline}
                    label={tr("designerContent.underTitle")}
                    onChange={(v) =>
                      edit((d) => {
                        const it = d.frontMatter[i];
                        if (it.type !== "fieldPage") return;
                        if (v) it.inline = true;
                        else delete it.inline;
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
              {tr(f.label)}
            </option>
          ))}
        </select>
        <button type="button" className="d-add" onClick={() => edit((d) => void d.frontMatter.push(newItem(adding, d)))}>
          {tr("designerContent.addPage")}
        </button>
      </div>
      {paragraphFields.length === 0 && <p className="d-note">{tr("designerContent.textPagesNeedField")}</p>}
    </>
  );
}

const BIB_STYLES: { value: BibStyle; label: MessageKey }[] = [
  { value: "ieee", label: "designerContent.bibIeee" },
  { value: "numeric", label: "designerContent.bibNumeric" },
  { value: "authorYear", label: "designerContent.bibAuthorYear" },
];

export function BibliographySection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const b = t.bibliography;
  return (
    <>
      <Row label={tr("designerContent.citationStyle")}>
        <Select value={b.style} options={options(BIB_STYLES, tr)} onChange={(v) => edit((d) => void (d.bibliography.style = v))} />
      </Row>
      <Row label={tr("designerContent.listHeading")} issues={issuesAt(issues, "bibliography.title")}>
        <input value={b.title} onChange={(e) => edit((d) => void (d.bibliography.title = e.target.value))} />
      </Row>
      <Row label={tr("designerContent.contents")}>
        <Toggle checked={b.inToc} label={tr("designerContent.listItInToc")} onChange={(v) => edit((d) => void (d.bibliography.inToc = v))} />
      </Row>
      <Row label={tr("designerContent.byLanguage")} hint={tr("designerContent.byLanguageHint")}>
        <Toggle
          checked={!!b.splitByLanguage}
          label={tr("designerContent.splitByLanguage")}
          onChange={(v) =>
            edit((d) => {
              if (v) d.bibliography.splitByLanguage = true;
              else delete d.bibliography.splitByLanguage;
            })
          }
        />
      </Row>
      <p className="d-note">{tr("designerContent.bibNote")}</p>
    </>
  );
}

export function SkeletonSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const level = t.structure.topLevel === "chapter" ? "chapter" : "section";
  return (
    <>
      <p className="d-note">{tr(level === "chapter" ? "designerContent.skeletonNoteChapters" : "designerContent.skeletonNoteSections")}</p>
      <div className="d-table">
        <div className="d-table-head skeleton">
          <span>{tr("common.title")}</span>
          <span>{tr("designerContent.colRequired")}</span>
          <span />
        </div>
        {t.skeleton.map((sec, i) => (
          <div key={i} className="d-table-row-wrap">
            <div className="d-table-row skeleton">
              <input value={sec.title} aria-label={tr("common.title")} onChange={(e) => edit((d) => void (d.skeleton[i].title = e.target.value))} />
              <input type="checkbox" aria-label={tr("designerContent.colRequired")} checked={sec.required} onChange={(e) => edit((d) => void (d.skeleton[i].required = e.target.checked))} />
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
        {tr(level === "chapter" ? "designerContent.addChapter" : "designerContent.addSection")}
      </button>
    </>
  );
}

const FRONT_STYLES: { value: FrontMatterStyle; label: MessageKey }[] = [
  { value: "ieeeJournal", label: "designerContent.styleIeeeJournal" },
  { value: "ieeeConference", label: "designerContent.styleIeeeConference" },
  { value: "elsevier", label: "designerContent.styleElsevier" },
  { value: "acm", label: "designerContent.styleAcm" },
  { value: "lncs", label: "designerContent.styleLncs" },
  { value: "revtex", label: "designerContent.styleRevtex" },
  { value: "custom", label: "designerContent.styleCustom" },
];

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

/** The template with a kit's imported files added, using the kit's class: the one the
 *  template expects, else the only one in the kit. */
export function withKitFiles(t: Template, added: string[]): Template {
  const d = structuredClone(t);
  if (!d.base) return d;
  for (const a of added) if (!d.base.files.includes(a)) d.base.files.push(a);
  const classes = added.filter((a) => a.endsWith(".cls")).map((a) => a.split("/").pop()!.replace(/\.cls$/, ""));
  const expected = d.base.kit?.required.find((r) => r.endsWith(".cls"))?.replace(/\.cls$/, "");
  if (expected && classes.includes(expected)) d.base.class = expected;
  else if (!expected && classes.length === 1) d.base.class = classes[0];
  return d;
}

/** Settings for templates built on a publisher's official class. */
export function PublisherSection({
  t,
  edit,
  issues,
  canImport,
  onImportKit,
}: SectionProps & { canImport: boolean; onImportKit: (zip: string) => Promise<void> }) {
  const tr = useT();
  const base = t.base;
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  if (!base) return null;
  const kit = base.kit;
  const haveNames = base.files.map((f) => f.split("/").pop()!);
  const kitMissing = kit ? kit.required.filter((r) => !haveNames.includes(r)) : [];
  const importZip = async () => {
    setProblem(null);
    const file = await open({ multiple: false, directory: false, filters: [{ name: tr("designerContent.kitFilter"), extensions: ["zip"] }] });
    if (!file) return;
    setBusy(true);
    try {
      await onImportKit(file);
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
      filters: [{ name: tr("designerContent.kitFilesFilter"), extensions: ["cls", "sty", "bst", "clo", "cfg", "def", "fd", "png", "jpg", "jpeg", "pdf", "eps", "otf", "ttf"] }],
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
          <strong>{tr(kitMissing.length ? "designerContent.kitNeeded" : "designerContent.kitImported", { kit: kit.name })}</strong>
          {kitMissing.length > 0 && (
            <>
              <p>{tr("designerContent.kitSeparately")}</p>
              <ol>
                <li>
                  {rich(tr("designerContent.downloadKitStep"), {
                    link: (
                      <button type="button" className="link" onClick={() => openUrl(kit.url)}>
                        {tr("designerContent.downloadKit")}
                      </button>
                    ),
                  })}
                </li>
                <li>{tr(canImport ? "designerContent.importZipStep" : "designerContent.importZipStepSave")}</li>
              </ol>
              <button type="button" className="primary" onClick={importZip} disabled={busy}>
                {tr(busy ? "designerContent.importing" : "designerContent.importKit")}
              </button>
            </>
          )}
        </div>
      )}
      <p className="d-note">{tr("designerContent.publisherNote")}</p>
      <Row label={tr("designerContent.class")} issues={issuesAt(issues, "base.class")}>
        <input className="mono" value={base.class} onChange={(e) => edit((d) => void (d.base && (d.base.class = e.target.value.trim())))} />
      </Row>
      <Row label={tr("designerContent.classOptions")} hint={tr("designerContent.commaSeparated")} issues={issuesAt(issues, "base.options")}>
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
      <Row label={tr("designerContent.titleBlockStyle")}>
        <Select value={base.frontMatter} options={options(FRONT_STYLES, tr)} onChange={(v) => edit((d) => void (d.base && (d.base.frontMatter = v)))} />
      </Row>
      {base.frontMatter === "custom" && (
        <>
          <Row label={tr("designerContent.titleBlockRecipe")} hint={tr("designerContent.latexWithPlaceholders")} issues={issuesAt(issues, "base.titleBlock")}>
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
                  {tr("designerContent.startFromExample")}
                </button>
              )}
              <details className="recipe-help">
                <summary>{tr("designerContent.placeholders")}</summary>
                <p>
                  {rich(tr("designerContent.recipeHelpFields"), {
                    basic: (
                      <>
                        <code>{"{title}"}</code> <code>{"{abstract}"}</code> <code>{"{keywords}"}</code>
                      </>
                    ),
                    corresponding: <code>{"{corresponding}"}</code>,
                    email: <code>{"{corresponding_email}"}</code>,
                  })}
                </p>
                <p>
                  {rich(tr("designerContent.recipeHelpAuthors"), {
                    block: <code>{"{#authors| \\and }…{/authors}"}</code>,
                    bar: <code>|</code>,
                    fields: (
                      <>
                        <code>{"{name}"}</code> <code>{"{email}"}</code> <code>{"{orcid}"}</code> <code>{"{membership}"}</code>{" "}
                        <code>{"{affiliation}"}</code> <code>{"{affiliations}"}</code> <code>{"{affnums}"}</code> <code>{"{n}"}</code>
                      </>
                    ),
                  })}
                </p>
                <p>
                  <code>{"{#affiliations|, }…{/affiliations}"}</code>: <code>{"{n}"}</code> <code>{"{name}"}</code>{" "}
                  <code>{"{emails}"}</code>. <code>{"{#keywords| \\sep }{keyword}{/keywords}"}</code>.
                </p>
                <p>{rich(tr("designerContent.recipeHelpOptional"), { block: <code>{"{?email}…{/email}"}</code> })}</p>
              </details>
            </div>
          </Row>
          <Row label={tr("designerContent.endBlock")} hint={tr("designerContent.endBlockHint")} issues={issuesAt(issues, "base.endBlock")}>
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
      <Row label={tr("designerContent.biographies")}>
        <Toggle
          checked={base.biographies}
          label={tr("designerContent.printBiographies")}
          onChange={(v) => edit((d) => void (d.base && (d.base.biographies = v)))}
        />
      </Row>
      <Row label={tr("designerContent.bibtexStyle")} issues={issuesAt(issues, "base.bibStyle")}>
        <input className="mono" placeholder={tr("designerContent.setByClass")} value={base.bibStyle} onChange={(e) => edit((d) => void (d.base && (d.base.bibStyle = e.target.value.trim())))} />
      </Row>
      <Row label={tr("designerContent.publisherFiles")} hint={tr("designerContent.publisherFilesHint")} issues={issuesAt(issues, "base.files")}>
        <div className="d-control">
          {base.files.length === 0 && <span className="muted">{tr("designerContent.noPublisherFiles")}</span>}
          {base.files.map((f, i) => (
            <span key={f} className="d-inline">
              <code>{f}</code>
              <button type="button" className="danger" onClick={() => edit((d) => void d.base?.files.splice(i, 1))}>
                {tr("common.remove")}
              </button>
            </span>
          ))}
          <span className="d-inline">
            <button type="button" className="d-add" onClick={importZip} disabled={busy}>
              {tr("designerContent.addImportKit")}
            </button>
            <button type="button" className="d-add" onClick={importFiles} disabled={!canImport} title={canImport ? "" : tr("designerContent.saveTemplateFirst")}>
              {tr("designerContent.importSingleFiles")}
            </button>
          </span>
          {!canImport && <span className="d-hint">{tr("designerContent.singleFilesNeedSave")}</span>}
          {problem && <span className="d-error">{problem}</span>}
        </div>
      </Row>
      <p className="d-note">
        {rich(tr("designerContent.otherJournals"), {
          title: <strong>{tr("designerContent.otherJournalsTitle")}</strong>,
          importTemplate: <em>{tr("designerContent.importTemplate")}</em>,
        })}
      </p>
    </>
  );
}
