// The template designer: panels that edit a draft template. Every change goes to
// the backend for validation and a live PDF preview (handled by App).
import { useCallback, useEffect, useState } from "react";
import * as api from "../api";
import type { Issue, Template, TemplateInfo } from "../api";
import { issuesAt } from "./controls";
import { useT, type MessageKey } from "../i18n";
import type { Edit } from "./common";
import { FontsSection, InfoSection, LanguageSection, PageSection, StructureSection } from "./PageSections";
import { CaptionsSection, HeaderFooterSection, HeadingsSection } from "./StyleSections";
import { BibliographySection, FieldsSection, FrontMatterSection, PublisherSection, SkeletonSection } from "./ContentSections";
import { TitlePageCanvas } from "./TitlePageCanvas";
import { SlidesSection } from "./SlidesSection";

// Labels are message keys, translated when drawn.
const SECTIONS = [
  { key: "info", label: "designer.navInfo", prefix: "meta" },
  { key: "publisher", label: "designer.navPublisher", prefix: "base" },
  { key: "slides", label: "designer.navSlides", prefix: "presentation" },
  { key: "language", label: "designer.language", prefix: "language" },
  { key: "page", label: "designer.navPage", prefix: "page" },
  { key: "fonts", label: "designer.navFonts", prefix: "fonts" },
  { key: "structure", label: "designer.navStructure", prefix: "structure" },
  { key: "headings", label: "designer.navHeadings", prefix: "headings" },
  { key: "headerFooter", label: "designer.navHeaderFooter", prefix: "headerFooter" },
  { key: "captions", label: "designer.navCaptions", prefix: "captions" },
  { key: "bibliography", label: "designer.navBibliography", prefix: "bibliography" },
  { key: "skeleton", label: "designer.navSkeleton", prefix: "skeleton" },
  { key: "fields", label: "designer.navFields", prefix: "fields" },
  { key: "frontMatter", label: "designer.navFrontMatter", prefix: "frontMatter" },
  { key: "titlePage", label: "designer.navTitlePage", prefix: "titlePage" },
] as const satisfies readonly { key: string; label: MessageKey; prefix: string }[];

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
  onNewBlank,
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
  /** Opens "New blank template". */
  onNewBlank: () => void;
  /** Imports a publisher kit (.zip), saving the template first when it has no file. */
  onImportKit: (zip: string) => Promise<void>;
  /** Opens the presentation theme picker. */
  onBrowseThemes: () => void;
}) {
  const t = useT();
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
          <span className="muted">{info.path ?? t("designer.builtInNote")}</span>
        </div>
        <div className="group">
          <button onClick={onNewBlank} title={t("designer.newBlankTitle")}>
            {t("designer.newBlank")}
          </button>
          <button onClick={onNewCopy} title={t("designer.newCopyTitle")}>
            {t("designer.newCopy")}
          </button>
          <button onClick={onSaveAs}>{t("designer.saveAs")}</button>
          <button onClick={onSave} disabled={!dirty || errorCount > 0} className="primary" title="Ctrl+S">
            {t("designer.saveTemplate")}
          </button>
        </div>
      </div>
      {errorCount > 0 && (
        <div className="designer-errors">
          {errorCount === 1 ? t("designer.problemsOne") : t("designer.problemsMany", { n: errorCount })}
        </div>
      )}
      <div className="designer-body">
        <nav className="designer-nav" aria-label={t("designer.navAria")}>
          {visible.map((s) => {
            const n = issuesAt(issues, s.prefix).filter((i) => i.severity === "error").length;
            return (
              <button key={s.key} className={section === s.key ? "active" : ""} onClick={() => setSection(s.key)}>
                {s.key === "skeleton" && draft.structure.topLevel === "section" ? t("designer.navSectionList") : t(s.label)}
                {n > 0 && <span className="badge">{n}</span>}
              </button>
            );
          })}
        </nav>
        <section className={`designer-panel${section === "titlePage" ? " wide" : ""}`}>
          <h3>
            {section === "skeleton" && draft.structure.topLevel === "section"
              ? t("designer.navSectionList")
              : t(SECTIONS.find((s) => s.key === section)!.label)}
          </h3>
          {section === "publisher" && <PublisherSection {...props} canImport={!!info.path && !needsSaveAs} onImportKit={onImportKit} />}
          {section === "info" && <InfoSection {...props} />}
          {section === "language" && <LanguageSection {...props} />}
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
