// The template designer: panels that edit a draft template. Every change goes to
// the backend for validation and a live PDF preview (handled by App).
import { useCallback, useEffect, useState } from "react";
import * as api from "../api";
import type { Issue, Template, TemplateInfo } from "../api";
import { issuesAt } from "./controls";
import type { Edit } from "./common";
import { FontsSection, InfoSection, PageSection, StructureSection } from "./PageSections";
import { CaptionsSection, HeaderFooterSection, HeadingsSection } from "./StyleSections";
import { BibliographySection, FieldsSection, FrontMatterSection, PublisherSection, SkeletonSection } from "./ContentSections";
import { TitlePageCanvas } from "./TitlePageCanvas";
import { SlidesSection } from "./SlidesSection";

const SECTIONS = [
  { key: "info", label: "Template info", prefix: "meta" },
  { key: "publisher", label: "Publisher class", prefix: "base" },
  { key: "slides", label: "Slides", prefix: "presentation" },
  { key: "page", label: "Page", prefix: "page" },
  { key: "fonts", label: "Fonts", prefix: "fonts" },
  { key: "structure", label: "Structure", prefix: "structure" },
  { key: "headings", label: "Headings", prefix: "headings" },
  { key: "headerFooter", label: "Headers & footers", prefix: "headerFooter" },
  { key: "captions", label: "Captions", prefix: "captions" },
  { key: "bibliography", label: "Bibliography", prefix: "bibliography" },
  { key: "skeleton", label: "Chapter list", prefix: "skeleton" },
  { key: "fields", label: "Fields", prefix: "fields" },
  { key: "frontMatter", label: "Front matter", prefix: "frontMatter" },
  { key: "titlePage", label: "Title page", prefix: "titlePage" },
] as const;

type SectionKey = (typeof SECTIONS)[number]["key"];

/** With a publisher class, the class decides these; their panels are hidden. */
const CLASS_DEFINED: SectionKey[] = ["page", "fonts", "structure", "headings", "headerFooter", "captions", "bibliography", "frontMatter", "titlePage"];

export function TemplateDesigner({
  info,
  draft,
  dirty,
  issues,
  metadata,
  needsSaveAs,
  onChange,
  onSave,
  onSaveAs,
  onNewCopy,
  onImportKit,
  onBrowseThemes,
}: {
  info: TemplateInfo;
  draft: Template;
  dirty: boolean;
  issues: Issue[];
  /** The open document's metadata, used to show real values on the title page. */
  metadata: api.Metadata;
  /** True for "New copy" drafts that don't have their own file yet. */
  needsSaveAs: boolean;
  onChange: (t: Template) => void;
  onSave: () => void;
  onSaveAs: () => void;
  onNewCopy: () => void;
  /** Imports a publisher kit (.zip), saving the template first when it has no file. */
  onImportKit: (zip: string) => Promise<void>;
  /** Opens the presentation theme picker. */
  onBrowseThemes: () => void;
}) {
  const usesClass = !!draft.base;
  const deck = draft.kind === "presentation";
  // Presentations: the theme decides the layout; only info, slides and fields apply.
  const visible = deck
    ? SECTIONS.filter((s) => ["info", "slides", "fields"].includes(s.key))
    : SECTIONS.filter((s) => s.key !== "slides" && (usesClass ? !CLASS_DEFINED.includes(s.key) : s.key !== "publisher"));
  const [chosen, setSection] = useState<SectionKey>("page");
  // Fall back to the first visible panel when the chosen one is hidden for this template.
  const section: SectionKey = visible.some((s) => s.key === chosen) ? chosen : visible[1]?.key ?? visible[0].key;
  const [systemFonts, setSystemFonts] = useState<string[]>([]);

  useEffect(() => {
    api.listSystemFonts().then(setSystemFonts, () => setSystemFonts([]));
  }, []);

  const edit: Edit = useCallback(
    (fn) => {
      const next = structuredClone(draft);
      fn(next);
      onChange(next);
    },
    [draft, onChange],
  );

  const props = { t: draft, edit, issues };
  const errorCount = issues.filter((i) => i.severity === "error").length;

  return (
    <div className="designer">
      <div className="designer-head">
        <div className="designer-title">
          <strong>
            {draft.meta.name}
            {dirty && " •"}
          </strong>
          <span className="muted">{info.path ?? "Built-in template (use Save as… to keep changes)"}</span>
        </div>
        <div className="group">
          <button onClick={onNewCopy} title="Start a new template from a copy of this one">
            New copy
          </button>
          <button onClick={onSaveAs}>Save as…</button>
          <button onClick={onSave} disabled={!dirty || errorCount > 0} className="primary" title="Ctrl+S">
            Save template
          </button>
        </div>
      </div>
      {errorCount > 0 && (
        <div className="designer-errors">
          {errorCount} problem{errorCount > 1 ? "s" : ""} to fix. The preview shows the last valid version.
        </div>
      )}
      <div className="designer-body">
        <nav className="designer-nav" aria-label="Template sections">
          {visible.map((s) => {
            const n = issuesAt(issues, s.prefix).filter((i) => i.severity === "error").length;
            return (
              <button key={s.key} className={section === s.key ? "active" : ""} onClick={() => setSection(s.key)}>
                {s.key === "skeleton" && draft.structure.topLevel === "section" ? "Section list" : s.label}
                {n > 0 && <span className="badge">{n}</span>}
              </button>
            );
          })}
        </nav>
        <section className={`designer-panel${section === "titlePage" ? " wide" : ""}`}>
          <h3>
            {section === "skeleton" && draft.structure.topLevel === "section"
              ? "Section list"
              : SECTIONS.find((s) => s.key === section)?.label}
          </h3>
          {section === "publisher" && <PublisherSection {...props} canImport={!!info.path && !needsSaveAs} onImportKit={onImportKit} />}
          {section === "info" && <InfoSection {...props} />}
          {section === "slides" && <SlidesSection {...props} onBrowseThemes={onBrowseThemes} />}
          {section === "page" && <PageSection {...props} />}
          {section === "fonts" && <FontsSection {...props} systemFonts={systemFonts} />}
          {section === "structure" && <StructureSection {...props} />}
          {section === "headings" && <HeadingsSection {...props} />}
          {section === "headerFooter" && <HeaderFooterSection {...props} />}
          {section === "captions" && <CaptionsSection {...props} />}
          {section === "bibliography" && <BibliographySection {...props} />}
          {section === "skeleton" && <SkeletonSection {...props} />}
          {section === "fields" && <FieldsSection {...props} />}
          {section === "frontMatter" && <FrontMatterSection {...props} />}
          {section === "titlePage" && (
            <TitlePageCanvas {...props} metadata={metadata} templatePath={info.path} canImport={!!info.path && !needsSaveAs} />
          )}
        </section>
      </div>
    </div>
  );
}
