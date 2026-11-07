// Designer panel for presentations: the beamer theme, colours, fonts, slide shape
// and the automatic outline slides.
import type { PresentationSettings } from "../bindings/PresentationSettings";
import { BEAMER_COLOR_THEMES, BEAMER_FONT_THEMES, BEAMER_THEMES } from "../editor/templateStyle";
import { ColorInput, PlaceholderInput, Row, Segmented, Select, Toggle, issuesAt, type PlaceholderOption } from "./controls";
import type { SectionProps, Translate } from "./common";
import { useT } from "../i18n";

const DEFAULTS: PresentationSettings = {
  theme: "Madrid",
  colorTheme: "",
  fontTheme: "",
  aspectRatio: "16:9",
  fontSizePt: 11,
  outline: false,
  sectionSlides: false,
  navigationSymbols: false,
  logo: "everySlide",
  footer: { style: "theme", left: "{author}", center: "{title}", right: "{page} / {pages}" },
};

const footerPlaceholders = (tr: Translate): PlaceholderOption[] => [
  { name: "author", description: tr("designer.phAuthor") },
  { name: "title", description: tr("designer.phTitle") },
  { name: "date", description: tr("designer.phDate") },
  { name: "institute", description: tr("designer.phInstitute") },
  { name: "section", description: tr("designer.phCurrentSection") },
  { name: "page", description: tr("designer.phSlideNumber") },
  { name: "pages", description: tr("designer.phSlideCount") },
];

export function SlidesSection({ t, edit, issues, onBrowseThemes }: SectionProps & { onBrowseThemes: () => void }) {
  const tr = useT();
  const p = t.presentation ?? DEFAULTS;
  const set = (patch: Partial<PresentationSettings>) => edit((d) => void (d.presentation = { ...(d.presentation ?? DEFAULTS), ...patch }));
  // Keep a theme typed in the template file (or chosen from the installed TeX) selectable.
  const themes = BEAMER_THEMES.includes(p.theme) ? BEAMER_THEMES : [p.theme, ...BEAMER_THEMES];
  const colorThemes = BEAMER_COLOR_THEMES.includes(p.colorTheme) || !p.colorTheme ? BEAMER_COLOR_THEMES : [p.colorTheme, ...BEAMER_COLOR_THEMES];
  return (
    <>
      <Row label={tr("designer.theme")} hint={tr("designer.themeHint")} issues={issuesAt(issues, "presentation.theme")}>
        <span className="d-inline">
          <Select value={p.theme} options={themes.map((v) => ({ value: v, label: v }))} onChange={(v) => set({ theme: v })} />
          <button type="button" onClick={onBrowseThemes} title={tr("designer.browseThemesTitle")}>
            {tr("designer.browseThemes")}
          </button>
        </span>
      </Row>
      <Row label={tr("designer.mainColour")} hint={tr("designer.mainColourHint")} issues={issuesAt(issues, "presentation.accentColor")}>
        <ColorInput value={p.accentColor} onChange={(v) => set({ accentColor: v })} />
      </Row>
      <Row label={tr("designer.colourTheme")} issues={issuesAt(issues, "presentation.colorTheme")}>
        <Select
          value={p.colorTheme}
          options={[{ value: "", label: tr("designer.themesOwn") }, ...colorThemes.map((v) => ({ value: v, label: v }))]}
          onChange={(v) => set({ colorTheme: v })}
        />
      </Row>
      <Row label={tr("designer.fontTheme")} issues={issuesAt(issues, "presentation.fontTheme")}>
        <Select
          value={p.fontTheme}
          options={[{ value: "", label: tr("designer.themesOwn") }, ...BEAMER_FONT_THEMES.map((v) => ({ value: v, label: v }))]}
          onChange={(v) => set({ fontTheme: v })}
        />
      </Row>
      <Row label={tr("designer.slideShape")}>
        <Segmented
          value={p.aspectRatio}
          options={[
            { value: "16:9", label: "16:9", title: tr("designer.widescreenTitle") },
            { value: "16:10", label: "16:10" },
            { value: "4:3", label: "4:3", title: tr("designer.classicTitle") },
          ]}
          onChange={(v) => set({ aspectRatio: v })}
        />
      </Row>
      <Row label={tr("designer.textSize")} issues={issuesAt(issues, "presentation.fontSizePt")}>
        <Select value={p.fontSizePt} options={[8, 9, 10, 11, 12, 14, 17, 20].map((v) => ({ value: v, label: `${v} pt` }))} onChange={(v) => set({ fontSizePt: v })} />
      </Row>
      <Row label={tr("designer.outline")}>
        <span className="d-stack">
          <Toggle checked={p.outline} onChange={(v) => set({ outline: v })} label={tr("designer.outlineAfterTitle")} />
          <Toggle checked={p.sectionSlides} onChange={(v) => set({ sectionSlides: v })} label={tr("designer.outlineEachSection")} />
        </span>
      </Row>
      <Row label={tr("designer.logo")} hint={tr("designer.logoHint")}>
        <Segmented
          value={p.logo}
          options={[
            { value: "everySlide", label: tr("designer.logoEverySlide"), title: tr("designer.logoEverySlideTitle") },
            { value: "titleSlide", label: tr("designer.logoTitleSlide") },
            { value: "off", label: tr("designer.logoOff") },
          ]}
          onChange={(v) => set({ logo: v })}
        />
      </Row>
      <Row label={tr("designer.footer")} hint={tr("designer.footerHint")}>
        <Segmented
          value={p.footer.style}
          options={[
            { value: "theme", label: tr("designer.footerThemeOwn"), title: tr("designer.footerThemeOwnTitle") },
            { value: "custom", label: tr("designer.custom") },
            { value: "none", label: tr("common.none") },
          ]}
          onChange={(v) => set({ footer: { ...p.footer, style: v } })}
        />
      </Row>
      {p.footer.style === "custom" &&
        (["left", "center", "right"] as const).map((side) => (
          <Row key={side} label={tr(({ left: "common.left", center: "designer.footerCentre", right: "common.right" } as const)[side])}>
            <PlaceholderInput value={p.footer[side]} placeholders={footerPlaceholders(tr)} onChange={(v) => set({ footer: { ...p.footer, [side]: v } })} />
          </Row>
        ))}
      <Row label={tr("designer.navigation")}>
        <Toggle checked={p.navigationSymbols} onChange={(v) => set({ navigationSymbols: v })} label={tr("designer.navigationSymbols")} />
      </Row>
    </>
  );
}
