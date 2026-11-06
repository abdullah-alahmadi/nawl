// Designer panel for presentations: the beamer theme, colours, fonts, slide shape
// and the automatic outline slides.
import type { PresentationSettings } from "../bindings/PresentationSettings";
import { BEAMER_COLOR_THEMES, BEAMER_FONT_THEMES, BEAMER_THEMES } from "../editor/templateStyle";
import { ColorInput, PlaceholderInput, Row, Segmented, Select, Toggle, issuesAt, type PlaceholderOption } from "./controls";
import type { SectionProps } from "./common";

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

const FOOTER_PLACEHOLDERS: PlaceholderOption[] = [
  { name: "author", description: "Presenters (short form)" },
  { name: "title", description: "Short title, else the title" },
  { name: "date", description: "Date or event" },
  { name: "institute", description: "Institution" },
  { name: "section", description: "Current section" },
  { name: "page", description: "Slide number" },
  { name: "pages", description: "Number of slides" },
];

export function SlidesSection({ t, edit, issues, onBrowseThemes }: SectionProps & { onBrowseThemes: () => void }) {
  const p = t.presentation ?? DEFAULTS;
  const set = (patch: Partial<PresentationSettings>) => edit((d) => void (d.presentation = { ...(d.presentation ?? DEFAULTS), ...patch }));
  // Keep a theme typed in the template file (or chosen from the installed TeX) selectable.
  const themes = BEAMER_THEMES.includes(p.theme) ? BEAMER_THEMES : [p.theme, ...BEAMER_THEMES];
  const colorThemes = BEAMER_COLOR_THEMES.includes(p.colorTheme) || !p.colorTheme ? BEAMER_COLOR_THEMES : [p.colorTheme, ...BEAMER_COLOR_THEMES];
  return (
    <>
      <Row label="Theme" hint="Beamer's layout: title bars, header and footer, navigation" issues={issuesAt(issues, "presentation.theme")}>
        <span className="d-inline">
          <Select value={p.theme} options={themes.map((v) => ({ value: v, label: v }))} onChange={(v) => set({ theme: v })} />
          <button type="button" onClick={onBrowseThemes} title="All themes and colours with previews, including your TeX installation's">
            Browse themes…
          </button>
        </span>
      </Row>
      <Row label="Main colour" hint="Instead of the theme's colours" issues={issuesAt(issues, "presentation.accentColor")}>
        <ColorInput value={p.accentColor} onChange={(v) => set({ accentColor: v })} />
      </Row>
      <Row label="Colour theme" issues={issuesAt(issues, "presentation.colorTheme")}>
        <Select
          value={p.colorTheme}
          options={[{ value: "", label: "The theme's own" }, ...colorThemes.map((v) => ({ value: v, label: v }))]}
          onChange={(v) => set({ colorTheme: v })}
        />
      </Row>
      <Row label="Font theme" issues={issuesAt(issues, "presentation.fontTheme")}>
        <Select
          value={p.fontTheme}
          options={[{ value: "", label: "The theme's own" }, ...BEAMER_FONT_THEMES.map((v) => ({ value: v, label: v }))]}
          onChange={(v) => set({ fontTheme: v })}
        />
      </Row>
      <Row label="Slide shape">
        <Segmented
          value={p.aspectRatio}
          options={[
            { value: "16:9", label: "16:9", title: "Widescreen, most projectors and screens today" },
            { value: "16:10", label: "16:10" },
            { value: "4:3", label: "4:3", title: "Classic" },
          ]}
          onChange={(v) => set({ aspectRatio: v })}
        />
      </Row>
      <Row label="Text size" issues={issuesAt(issues, "presentation.fontSizePt")}>
        <Select value={p.fontSizePt} options={[8, 9, 10, 11, 12, 14, 17, 20].map((v) => ({ value: v, label: `${v} pt` }))} onChange={(v) => set({ fontSizePt: v })} />
      </Row>
      <Row label="Outline">
        <span className="d-stack">
          <Toggle checked={p.outline} onChange={(v) => set({ outline: v })} label="Outline slide after the title slide" />
          <Toggle checked={p.sectionSlides} onChange={(v) => set({ sectionSlides: v })} label="Outline slide at each section, highlighting it" />
        </span>
      </Row>
      <Row label="Logo" hint="The image itself is chosen in Document info (Logo)">
        <Segmented
          value={p.logo}
          options={[
            { value: "everySlide", label: "Every slide", title: "On the title slide and in the corner of every slide" },
            { value: "titleSlide", label: "Title slide" },
            { value: "off", label: "Off" },
          ]}
          onChange={(v) => set({ logo: v })}
        />
      </Row>
      <Row label="Footer" hint="The line at the bottom of each slide">
        <Segmented
          value={p.footer.style}
          options={[
            { value: "theme", label: "Theme's own", title: "What the theme shows (Madrid: presenter, title, date, slide number)" },
            { value: "custom", label: "Custom" },
            { value: "none", label: "None" },
          ]}
          onChange={(v) => set({ footer: { ...p.footer, style: v } })}
        />
      </Row>
      {p.footer.style === "custom" &&
        (["left", "center", "right"] as const).map((side) => (
          <Row key={side} label={{ left: "Left", center: "Centre", right: "Right" }[side]}>
            <PlaceholderInput value={p.footer[side]} placeholders={FOOTER_PLACEHOLDERS} onChange={(v) => set({ footer: { ...p.footer, [side]: v } })} />
          </Row>
        ))}
      <Row label="Navigation">
        <Toggle checked={p.navigationSymbols} onChange={(v) => set({ navigationSymbols: v })} label="Beamer's navigation symbols in the corner" />
      </Row>
    </>
  );
}
