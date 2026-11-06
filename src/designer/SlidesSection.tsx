// Designer panel for presentations: the beamer theme, colours, fonts, slide shape
// and the automatic outline slides.
import type { PresentationSettings } from "../bindings/PresentationSettings";
import { BEAMER_COLOR_THEMES, BEAMER_FONT_THEMES, BEAMER_THEMES } from "../editor/templateStyle";
import { Row, Segmented, Select, Toggle, issuesAt } from "./controls";
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
};

export function SlidesSection({ t, edit, issues }: SectionProps) {
  const p = t.presentation ?? DEFAULTS;
  const set = (patch: Partial<PresentationSettings>) => edit((d) => void (d.presentation = { ...(d.presentation ?? DEFAULTS), ...patch }));
  // Keep a theme typed in the template file selectable even if it's not in the list.
  const themes = BEAMER_THEMES.includes(p.theme) ? BEAMER_THEMES : [p.theme, ...BEAMER_THEMES];
  return (
    <>
      <Row label="Theme" hint="Beamer's layout: title bars, header and footer, navigation" issues={issuesAt(issues, "presentation.theme")}>
        <Select value={p.theme} options={themes.map((v) => ({ value: v, label: v }))} onChange={(v) => set({ theme: v })} />
      </Row>
      <Row label="Colour theme" issues={issuesAt(issues, "presentation.colorTheme")}>
        <Select
          value={p.colorTheme}
          options={[{ value: "", label: "The theme's own" }, ...BEAMER_COLOR_THEMES.map((v) => ({ value: v, label: v }))]}
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
      <Row label="Navigation">
        <Toggle checked={p.navigationSymbols} onChange={(v) => set({ navigationSymbols: v })} label="Beamer's navigation symbols in the corner" />
      </Row>
    </>
  );
}
