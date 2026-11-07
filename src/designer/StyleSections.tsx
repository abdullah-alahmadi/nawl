// Designer panels: headings and headers/footers.
import { useState } from "react";
import type { HeadingStyle } from "../bindings/HeadingStyle";
import type { PageStyle } from "../bindings/PageStyle";
import type { TextStyle } from "../bindings/TextStyle";
import { ColorInput, NumberInput, PlaceholderInput, Row, Segmented, Toggle, issuesAt } from "./controls";
import { dynamicPlaceholders, fieldPlaceholders, headingNames, type SectionProps } from "./common";
import { translate, useT, type MessageKey, type UiLanguage } from "../i18n";

/** The samples show text in the template's language (the document's, not the interface's). */
function sampleLanguage(t: SectionProps["t"]): UiLanguage {
  return t.language?.main === "arabic" ? "ar" : "en";
}

/** Pixels per point at the preview's scale. */
const PX_PER_PT = 1.1;

function textCss(s: TextStyle): React.CSSProperties {
  return {
    fontSize: s.sizePt * PX_PER_PT,
    fontWeight: s.bold ? 700 : 400,
    fontStyle: s.italic ? "italic" : "normal",
    fontVariant: s.smallCaps ? "small-caps" : "normal",
    textTransform: s.uppercase ? "uppercase" : "none",
    textAlign: s.align,
    color: s.color ?? "#111",
  };
}

/** Instant on-screen approximation of a heading; the PDF preview is exact. */
function HeadingSample({ style, level, numbered, lang }: { style: HeadingStyle; level: number; numbered: boolean; lang: UiLanguage }) {
  const tr = useT();
  const sample = (key: MessageKey) => translate(lang, key);
  const number = ["2", "2.1", "2.1.3", "2.1.3.1"][level];
  const label = numbered ? style.label.replace(/\{n\}/g, number) : "";
  const css = textCss(style.text);
  return (
    <div className="d-sample" aria-label={tr("designer.headingPreview")} dir={lang === "ar" ? "rtl" : "ltr"}>
      <div className="d-sample-text">{sample("designer.sampleBefore")}</div>
      <div style={{ ...css, marginTop: style.spaceBeforePt * PX_PER_PT, marginBottom: style.spaceAfterPt * PX_PER_PT }}>
        {style.layout === "display" ? (
          <>
            {label && <div style={{ marginBottom: style.labelGapPt * PX_PER_PT }}>{label}</div>}
            <div>{sample("designer.sampleHeading")}</div>
          </>
        ) : (
          <div>
            {label && <span style={{ marginInlineEnd: style.labelGapPt * PX_PER_PT }}>{label}</span>}
            {sample("designer.sampleHeading")}
          </div>
        )}
      </div>
      <div className="d-sample-text">{sample("designer.sampleAfter")}</div>
    </div>
  );
}

export function TextStyleControls({
  style,
  onChange,
  path,
  issues,
}: {
  style: TextStyle;
  onChange: (fn: (s: TextStyle) => void) => void;
  path: string;
  issues: SectionProps["issues"];
}) {
  const tr = useT();
  return (
    <>
      <Row label={tr("common.size")} issues={issuesAt(issues, `${path}.sizePt`)}>
        <NumberInput value={style.sizePt} unit="pt" min={4} step={0.5} onChange={(v) => onChange((s) => void (s.sizePt = v))} />
      </Row>
      <Row label={tr("designer.style")}>
        <span className="d-inline">
          <Toggle checked={style.bold} onChange={(v) => onChange((s) => void (s.bold = v))} label={tr("common.bold")} />
          <Toggle checked={style.italic} onChange={(v) => onChange((s) => void (s.italic = v))} label={tr("common.italic")} />
          <Toggle checked={style.smallCaps} onChange={(v) => onChange((s) => void (s.smallCaps = v))} label={tr("designer.smallCaps")} />
          <Toggle checked={style.uppercase} onChange={(v) => onChange((s) => void (s.uppercase = v))} label={tr("designer.allCaps")} />
        </span>
      </Row>
      <Row label={tr("designer.alignment")}>
        <Segmented
          value={style.align}
          options={[
            { value: "left", label: tr("common.left") },
            { value: "center", label: tr("common.center") },
            { value: "right", label: tr("common.right") },
          ]}
          onChange={(v) => onChange((s) => void (s.align = v))}
        />
      </Row>
      <Row label={tr("common.color")} issues={issuesAt(issues, `${path}.color`)}>
        <ColorInput
          value={style.color}
          onChange={(v) =>
            onChange((s) => {
              if (v) s.color = v;
              else delete s.color;
            })
          }
        />
      </Row>
    </>
  );
}

export function HeadingsSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const [level, setLevel] = useState(0);
  const names = headingNames(t, tr);
  const style = t.headings.levels[level];
  if (!style) return <p className="d-error">{tr("designer.headingLevelMissing", { n: level + 1 })}</p>;
  const path = `headings.levels[${level}]`;
  const set = (fn: (h: HeadingStyle) => void) => edit((d) => fn(d.headings.levels[level]));
  const numbered = level < t.structure.numberedDepth;
  return (
    <>
      <Segmented
        value={String(level)}
        options={names.map((n, i) => ({
          value: String(i),
          label: `${i + 1}. ${n}`,
          title: issuesAt(issues, `headings.levels[${i}]`).length ? tr("designer.hasProblems") : undefined,
        }))}
        onChange={(v) => setLevel(Number(v))}
      />
      <HeadingSample style={style} level={level} numbered={numbered} lang={sampleLanguage(t)} />
      <TextStyleControls style={style.text} path={`${path}.text`} issues={issues} onChange={(fn) => set((h) => fn(h.text))} />
      <Row label={tr("designer.layout")}>
        <Segmented
          value={style.layout}
          options={[
            { value: "inline", label: tr("designer.layoutInline") },
            { value: "display", label: tr("designer.layoutDisplay") },
          ]}
          onChange={(v) => set((h) => void (h.layout = v))}
        />
      </Row>
      <Row
        label={tr("designer.numberLabel")}
        hint={tr(numbered ? "designer.numberLabelHint" : "designer.notNumbered")}
        issues={issuesAt(issues, `${path}.label`)}
      >
        <PlaceholderInput
          value={style.label}
          onChange={(v) => set((h) => void (h.label = v))}
          placeholders={[{ name: "n", description: tr("designer.phHeadingNumber") }]}
        />
      </Row>
      <Row label={tr("designer.gapAfterNumber")} issues={issuesAt(issues, `${path}.labelGapPt`)}>
        <NumberInput value={style.labelGapPt} unit="pt" min={0} onChange={(v) => set((h) => void (h.labelGapPt = v))} />
      </Row>
      <Row label={tr("designer.spaceBeforeAfter")}>
        <span className="d-inline">
          <NumberInput value={style.spaceBeforePt} unit="pt" min={0} onChange={(v) => set((h) => void (h.spaceBeforePt = v))} />
          <NumberInput value={style.spaceAfterPt} unit="pt" min={0} onChange={(v) => set((h) => void (h.spaceAfterPt = v))} />
        </span>
      </Row>
    </>
  );
}

// Labels and hints are message keys, translated when drawn.
const PAGE_STYLES = [
  { key: "main", label: "designer.pageStyleMain", hint: "designer.pageStyleMainHint" },
  { key: "chapterOpening", label: "designer.pageStyleChapterOpening", hint: "designer.pageStyleChapterOpeningHint" },
  { key: "frontMatter", label: "designer.navFrontMatter", hint: "designer.pageStyleFrontMatterHint" },
] as const satisfies readonly { key: string; label: MessageKey; hint: MessageKey }[];

export function HeaderFooterSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const [which, setWhich] = useState<(typeof PAGE_STYLES)[number]["key"]>("main");
  const style = t.headerFooter[which];
  const base = `headerFooter.${which}`;
  const placeholders = [...dynamicPlaceholders(tr), ...fieldPlaceholders(t)];
  const set = (fn: (s: PageStyle) => void) => edit((d) => fn(d.headerFooter[which]));
  const info = PAGE_STYLES.find((p) => p.key === which)!;

  const slots = (part: "header" | "footer") => (
    <div className="d-slots">
      {(["left", "center", "right"] as const).map((pos) => (
        <label key={pos}>
          <span className="d-hint">{tr(`common.${pos}`)}</span>
          <PlaceholderInput
            value={style[part][pos]}
            onChange={(v) => set((s) => void (s[part][pos] = v))}
            placeholders={placeholders}
          />
          {issuesAt(issues, `${base}.${part}.${pos}`).map((i, n) => (
            <div key={n} className="d-error">
              {i.message}
            </div>
          ))}
        </label>
      ))}
    </div>
  );

  return (
    <>
      <Segmented value={which} options={PAGE_STYLES.map((p) => ({ value: p.key, label: tr(p.label) }))} onChange={setWhich} />
      <p className="d-note">
        {tr(info.hint)}. {tr("designer.chipHint")}
      </p>
      <h4 className="d-subhead">{tr("designer.header")}</h4>
      {slots("header")}
      <Row label={tr("designer.lineUnderHeader")} issues={issuesAt(issues, `${base}.headerRulePt`)}>
        <NumberInput value={style.headerRulePt} unit={tr("designer.ptZeroNone")} min={0} step={0.1} onChange={(v) => set((s) => void (s.headerRulePt = v))} />
      </Row>
      <h4 className="d-subhead">{tr("designer.footer")}</h4>
      {slots("footer")}
      <Row label={tr("designer.lineAboveFooter")} issues={issuesAt(issues, `${base}.footerRulePt`)}>
        <NumberInput value={style.footerRulePt} unit={tr("designer.ptZeroNone")} min={0} step={0.1} onChange={(v) => set((s) => void (s.footerRulePt = v))} />
      </Row>
    </>
  );
}

export function CaptionsSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const lang = sampleLanguage(t);
  const c = t.captions;
  const set = (fn: (c: SectionProps["t"]["captions"]) => void) => edit((d) => fn(d.captions));
  const sep = { colon: ": ", period: ". ", dash: " — ", newline: "" }[c.separator];
  const number = t.structure.topLevel === "chapter" && t.structure.perChapterNumbering ? "3.1" : "4";
  const sample = (label: string, text: string) => (
    <div
      style={{
        fontSize: { footnote: 12, small: 13.5, normal: 15 }[c.size],
        textAlign: c.align === "left" ? "left" : c.align === "justified" ? "justify" : "center",
      }}
    >
      <span style={{ fontWeight: c.labelBold ? 700 : 400, display: c.separator === "newline" ? "block" : "inline" }}>
        {label} {number}
        {sep}
      </span>
      {text}
    </div>
  );
  return (
    <>
      <div className="d-sample" aria-label={tr("designer.captionPreview")} dir={lang === "ar" ? "rtl" : "ltr"}>
        <div className="d-sample-figure">{translate(lang, "designer.sampleImage")}</div>
        {sample(c.figureLabel, translate(lang, "designer.sampleCaption"))}
      </div>
      <Row label={tr("designer.figureLabel")} issues={issuesAt(issues, "captions.figureLabel")}>
        <input value={c.figureLabel} onChange={(e) => set((x) => void (x.figureLabel = e.target.value))} />
      </Row>
      <Row label={tr("designer.tableLabel")} issues={issuesAt(issues, "captions.tableLabel")}>
        <input value={c.tableLabel} onChange={(e) => set((x) => void (x.tableLabel = e.target.value))} />
      </Row>
      <Row label={tr("designer.afterNumber")}>
        <Segmented
          value={c.separator}
          options={[
            { value: "colon", label: tr("designer.sepColon") },
            { value: "period", label: tr("designer.sepPeriod") },
            { value: "dash", label: tr("designer.sepDash") },
            { value: "newline", label: tr("designer.sepNewline") },
          ]}
          onChange={(v) => set((x) => void (x.separator = v))}
        />
      </Row>
      <Row label={tr("designer.label")}>
        <Toggle
          checked={c.labelBold}
          onChange={(v) => set((x) => void (x.labelBold = v))}
          label={tr("designer.labelBold", { label: c.figureLabel })}
        />
      </Row>
      <Row label={tr("common.size")}>
        <Segmented
          value={c.size}
          options={[
            { value: "footnote", label: tr("designer.captionSmaller") },
            { value: "small", label: tr("designer.captionSmall") },
            { value: "normal", label: tr("designer.captionSameAsText") },
          ]}
          onChange={(v) => set((x) => void (x.size = v))}
        />
      </Row>
      <Row label={tr("designer.alignment")}>
        <Segmented
          value={c.align}
          options={[
            { value: "center", label: tr("designer.centered") },
            { value: "left", label: tr("common.left") },
            { value: "justified", label: tr("designer.justified") },
          ]}
          onChange={(v) => set((x) => void (x.align = v))}
        />
      </Row>
      <p className="d-note">{tr("designer.captionsNote")}</p>
    </>
  );
}
