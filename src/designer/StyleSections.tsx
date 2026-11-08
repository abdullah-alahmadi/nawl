// Designer panels: headings and headers/footers.
import { useEffect, useState } from "react";
import * as api from "../api";
import { FontPicker } from "./PageSections";
import type { HeadingStyle } from "../bindings/HeadingStyle";
import type { PartStyle } from "../bindings/PartStyle";
import type { PageStyle } from "../bindings/PageStyle";
import type { TextStyle } from "../bindings/TextStyle";
import { ColorInput, NumberInput, PlaceholderInput, Row, Segmented, Toggle, issuesAt } from "./controls";
import { dynamicPlaceholders, fieldPlaceholders, headingNames, type SectionProps } from "./common";
import { translate, useT, type MessageKey, type UiLanguage } from "../i18n";
import { cssFontStack, pageFontStack } from "../editor/templateStyle";
import { docLanguage, localDigits } from "../editor/language";

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

/** Instant on-screen approximation of a heading; the PDF preview is exact. It uses
 *  the level's own font, else the template's page font, and the template's digits. */
function HeadingSample({ t, style, level, numbered, lang }: { t: SectionProps["t"]; style: HeadingStyle; level: number; numbered: boolean; lang: UiLanguage }) {
  const tr = useT();
  const sample = (key: MessageKey) => translate(lang, key);
  const number = localDigits(["2", "2.1", "2.1.3", "2.1.3.1"][level], docLanguage(t).digits);
  const label = numbered ? style.label.replace(/\{n\}/g, number) : "";
  const css = { ...textCss(style.text), fontFamily: style.text.font ? cssFontStack(style.text.font) : pageFontStack(t) };
  return (
    <div className="d-sample" aria-label={tr("designer.headingPreview")} dir={lang === "ar" ? "rtl" : "ltr"} style={{ fontFamily: pageFontStack(t) }}>
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

let systemFontList: Promise<string[]> | null = null;

/** The computer's fonts, for "installed font" (asked for once). */
function useSystemFonts(): string[] {
  const [fonts, setFonts] = useState<string[]>([]);
  useEffect(() => {
    systemFontList ??= api.listSystemFonts().catch(() => []);
    void systemFontList.then(setFonts);
  }, []);
  return fonts;
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
  const systemFonts = useSystemFonts();
  return (
    <>
      <Row label={tr("designer.styleFont")} hint={tr("designer.styleFontHint")} issues={issuesAt(issues, `${path}.font`)}>
        <FontPicker
          value={style.font}
          systemFonts={systemFonts}
          inheritLabel={tr("designer.sameAsText")}
          onChange={(f) =>
            onChange((s) => {
              if (f) s.font = f;
              else delete s.font;
            })
          }
        />
      </Row>
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
  // Parts group chapters (in a project): only for templates with chapters.
  const withParts = t.structure.topLevel === "chapter" && !t.base;
  const tabs = (
    <Segmented
      value={level < 0 ? "part" : String(level)}
      options={[
        ...names.map((n, i) => ({
          value: String(i),
          label: `${i + 1}. ${n}`,
          title: issuesAt(issues, `headings.levels[${i}]`).length ? tr("designer.hasProblems") : undefined,
        })),
        ...(withParts ? [{ value: "part", label: tr("designer.partTab"), title: tr("designer.partTabTitle") }] : []),
      ]}
      onChange={(v) => setLevel(v === "part" ? -1 : Number(v))}
    />
  );
  if (level < 0 && withParts) {
    return (
      <>
        {tabs}
        <PartControls t={t} edit={edit} issues={issues} />
      </>
    );
  }
  const style = t.headings.levels[level];
  if (!style) return <p className="d-error">{tr("designer.headingLevelMissing", { n: level + 1 })}</p>;
  const path = `headings.levels[${level}]`;
  const set = (fn: (h: HeadingStyle) => void) => edit((d) => fn(d.headings.levels[level]));
  const numbered = level < t.structure.numberedDepth;
  return (
    <>
      {tabs}
      <HeadingSample t={t} style={style} level={level} numbered={numbered} lang={sampleLanguage(t)} />
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

/** A part's number in words, as the PDF writes it ("الأول", "One"). */
const PART_WORDS = { ar: ["الأول", "الثاني", "الثالث"], en: ["One", "Two", "Three"] };

/** The part heading ("الباب الأول"): the base class's plain \part, or a style of its own. */
function PartControls({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const part = t.headings.part;
  const lang = sampleLanguage(t);
  const set = (fn: (p: PartStyle) => void) => edit((d) => d.headings.part && fn(d.headings.part));
  const enable = (on: boolean) =>
    edit((d) => {
      if (!on) delete d.headings.part;
      else
        d.headings.part = {
          text: { ...d.headings.levels[0].text, sizePt: d.headings.levels[0].text.sizePt + 6, bold: true, align: "center" },
          label: lang === "ar" ? "الباب {n}" : "Part {n}",
          numberWords: true,
          ownPage: true,
          inToc: true,
          labelGapPt: 12,
        };
    });
  return (
    <>
      <Row label={tr("designer.partCustom")} hint={tr("designer.partCustomHint")}>
        <Toggle checked={!!part} onChange={enable} label={tr("designer.partCustomToggle")} />
      </Row>
      {part && (
        <>
          <div className="d-sample" aria-label={tr("designer.headingPreview")} dir={lang === "ar" ? "rtl" : "ltr"} style={{ fontFamily: pageFontStack(t) }}>
            <div style={{ ...textCss(part.text), fontFamily: part.text.font ? cssFontStack(part.text.font) : pageFontStack(t) }}>
              <div style={{ marginBottom: part.labelGapPt * PX_PER_PT }}>
                {part.label.replace(/\{n\}/g, part.numberWords ? PART_WORDS[lang][0] : localDigits("1", docLanguage(t).digits))}
              </div>
              <div>{translate(lang, "designer.partSampleTitle")}</div>
            </div>
          </div>
          <TextStyleControls style={part.text} path="headings.part.text" issues={issues} onChange={(fn) => set((p) => fn(p.text))} />
          <Row label={tr("designer.numberLabel")} hint={tr("designer.partLabelHint")} issues={issuesAt(issues, "headings.part.label")}>
            <PlaceholderInput value={part.label} onChange={(v) => set((p) => void (p.label = v))} placeholders={[{ name: "n", description: tr("designer.phPartNumber") }]} />
          </Row>
          <Row label={tr("designer.partOptions")}>
            <span className="d-inline">
              <Toggle checked={part.numberWords} onChange={(v) => set((p) => void (p.numberWords = v))} label={tr("designer.partNumberWords")} />
              <Toggle checked={part.ownPage} onChange={(v) => set((p) => void (p.ownPage = v))} label={tr("designer.partOwnPage")} />
              <Toggle checked={part.inToc} onChange={(v) => set((p) => void (p.inToc = v))} label={tr("designer.partInToc")} />
            </span>
          </Row>
          <Row label={tr("designer.gapAfterNumber")} issues={issuesAt(issues, "headings.part.labelGapPt")}>
            <NumberInput value={part.labelGapPt} unit="pt" min={0} onChange={(v) => set((p) => void (p.labelGapPt = v))} />
          </Row>
        </>
      )}
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
  const number = localDigits(t.structure.topLevel === "chapter" && t.structure.perChapterNumbering ? "3.1" : "4", docLanguage(t).digits);
  const sample = (label: string, text: string) => (
    <div
      style={{
        fontFamily: pageFontStack(t),
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
