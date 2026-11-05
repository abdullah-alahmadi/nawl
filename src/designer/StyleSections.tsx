// Designer panels: headings and headers/footers.
import { useState } from "react";
import type { HeadingStyle } from "../bindings/HeadingStyle";
import type { PageStyle } from "../bindings/PageStyle";
import type { TextStyle } from "../bindings/TextStyle";
import { ColorInput, NumberInput, PlaceholderInput, Row, Segmented, Toggle, issuesAt } from "./controls";
import { DYNAMIC_PLACEHOLDERS, fieldPlaceholders, headingNames, type SectionProps } from "./common";

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
function HeadingSample({ style, level, numbered }: { style: HeadingStyle; level: number; numbered: boolean }) {
  const number = ["2", "2.1", "2.1.3", "2.1.3.1"][level];
  const label = numbered ? style.label.replace(/\{n\}/g, number) : "";
  const css = textCss(style.text);
  return (
    <div className="d-sample" aria-label="Heading preview">
      <div className="d-sample-text">Previous paragraph text ends here.</div>
      <div style={{ ...css, marginTop: style.spaceBeforePt * PX_PER_PT, marginBottom: style.spaceAfterPt * PX_PER_PT }}>
        {style.layout === "display" ? (
          <>
            {label && <div style={{ marginBottom: style.labelGapPt * PX_PER_PT }}>{label}</div>}
            <div>Sample Heading</div>
          </>
        ) : (
          <div>
            {label && <span style={{ marginRight: style.labelGapPt * PX_PER_PT }}>{label}</span>}
            Sample Heading
          </div>
        )}
      </div>
      <div className="d-sample-text">The next paragraph starts here and continues for a while.</div>
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
  return (
    <>
      <Row label="Size" issues={issuesAt(issues, `${path}.sizePt`)}>
        <NumberInput value={style.sizePt} unit="pt" min={4} step={0.5} onChange={(v) => onChange((s) => void (s.sizePt = v))} />
      </Row>
      <Row label="Style">
        <span className="d-inline">
          <Toggle checked={style.bold} onChange={(v) => onChange((s) => void (s.bold = v))} label="Bold" />
          <Toggle checked={style.italic} onChange={(v) => onChange((s) => void (s.italic = v))} label="Italic" />
          <Toggle checked={style.smallCaps} onChange={(v) => onChange((s) => void (s.smallCaps = v))} label="Small caps" />
          <Toggle checked={style.uppercase} onChange={(v) => onChange((s) => void (s.uppercase = v))} label="ALL CAPS" />
        </span>
      </Row>
      <Row label="Alignment">
        <Segmented
          value={style.align}
          options={[
            { value: "left", label: "Left" },
            { value: "center", label: "Center" },
            { value: "right", label: "Right" },
          ]}
          onChange={(v) => onChange((s) => void (s.align = v))}
        />
      </Row>
      <Row label="Colour" issues={issuesAt(issues, `${path}.color`)}>
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
  const [level, setLevel] = useState(0);
  const names = headingNames(t);
  const style = t.headings.levels[level];
  if (!style) return <p className="d-error">This template doesn't define heading level {level + 1}.</p>;
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
          title: issuesAt(issues, `headings.levels[${i}]`).length ? "Has problems" : undefined,
        }))}
        onChange={(v) => setLevel(Number(v))}
      />
      <HeadingSample style={style} level={level} numbered={numbered} />
      <TextStyleControls style={style.text} path={`${path}.text`} issues={issues} onChange={(fn) => set((h) => fn(h.text))} />
      <Row label="Layout">
        <Segmented
          value={style.layout}
          options={[
            { value: "inline", label: "Number and title on one line" },
            { value: "display", label: "Number above title" },
          ]}
          onChange={(v) => set((h) => void (h.layout = v))}
        />
      </Row>
      <Row
        label="Number label"
        hint={numbered ? "{n} is the number" : "Not numbered (see Structure)"}
        issues={issuesAt(issues, `${path}.label`)}
      >
        <PlaceholderInput
          value={style.label}
          onChange={(v) => set((h) => void (h.label = v))}
          placeholders={[{ name: "n", description: "The heading number" }]}
        />
      </Row>
      <Row label="Gap after number" issues={issuesAt(issues, `${path}.labelGapPt`)}>
        <NumberInput value={style.labelGapPt} unit="pt" min={0} onChange={(v) => set((h) => void (h.labelGapPt = v))} />
      </Row>
      <Row label="Space before / after">
        <span className="d-inline">
          <NumberInput value={style.spaceBeforePt} unit="pt" min={0} onChange={(v) => set((h) => void (h.spaceBeforePt = v))} />
          <NumberInput value={style.spaceAfterPt} unit="pt" min={0} onChange={(v) => set((h) => void (h.spaceAfterPt = v))} />
        </span>
      </Row>
    </>
  );
}

const PAGE_STYLES = [
  { key: "main", label: "Main text", hint: "Ordinary pages of the report" },
  { key: "chapterOpening", label: "Chapter openings", hint: "Pages where a chapter starts" },
  { key: "frontMatter", label: "Front matter", hint: "Abstract, table of contents…" },
] as const;

export function HeaderFooterSection({ t, edit, issues }: SectionProps) {
  const [which, setWhich] = useState<(typeof PAGE_STYLES)[number]["key"]>("main");
  const style = t.headerFooter[which];
  const base = `headerFooter.${which}`;
  const placeholders = [...DYNAMIC_PLACEHOLDERS, ...fieldPlaceholders(t)];
  const set = (fn: (s: PageStyle) => void) => edit((d) => fn(d.headerFooter[which]));
  const info = PAGE_STYLES.find((p) => p.key === which)!;

  const slots = (part: "header" | "footer") => (
    <div className="d-slots">
      {(["left", "center", "right"] as const).map((pos) => (
        <label key={pos}>
          <span className="d-hint">{pos}</span>
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
      <Segmented value={which} options={PAGE_STYLES.map((p) => ({ value: p.key, label: p.label }))} onChange={setWhich} />
      <p className="d-note">{info.hint}. Click a chip to insert it where the cursor is.</p>
      <h4 className="d-subhead">Header</h4>
      {slots("header")}
      <Row label="Line under header" issues={issuesAt(issues, `${base}.headerRulePt`)}>
        <NumberInput value={style.headerRulePt} unit="pt (0 = none)" min={0} step={0.1} onChange={(v) => set((s) => void (s.headerRulePt = v))} />
      </Row>
      <h4 className="d-subhead">Footer</h4>
      {slots("footer")}
      <Row label="Line above footer" issues={issuesAt(issues, `${base}.footerRulePt`)}>
        <NumberInput value={style.footerRulePt} unit="pt (0 = none)" min={0} step={0.1} onChange={(v) => set((s) => void (s.footerRulePt = v))} />
      </Row>
    </>
  );
}
