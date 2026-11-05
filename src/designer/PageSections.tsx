// Designer panels: template info, page, fonts, structure.
import type { BuiltinFont } from "../bindings/BuiltinFont";
import type { FontChoice } from "../bindings/FontChoice";
import type { Paper } from "../bindings/Paper";
import type { LineSpacing } from "../bindings/LineSpacing";
import type { PageNumberStyle } from "../bindings/PageNumberStyle";
import { NumberInput, Row, Select, Toggle, issuesAt } from "./controls";
import type { SectionProps } from "./common";

export function InfoSection({ t, edit, issues }: SectionProps) {
  return (
    <>
      <Row label="Name" issues={issuesAt(issues, "meta.name")}>
        <input value={t.meta.name} onChange={(e) => edit((d) => void (d.meta.name = e.target.value))} />
      </Row>
      <Row label="ID" hint="Also the .cls file name" issues={issuesAt(issues, "meta.id")}>
        <input
          className="mono"
          value={t.meta.id}
          onChange={(e) => edit((d) => void (d.meta.id = e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, "-")))}
        />
      </Row>
      <Row label="Description">
        <textarea rows={3} value={t.meta.description} onChange={(e) => edit((d) => void (d.meta.description = e.target.value))} />
      </Row>
      <Row label="Version">
        <input value={t.meta.version} onChange={(e) => edit((d) => void (d.meta.version = e.target.value))} />
      </Row>
      <Row label="Author">
        <input
          value={t.meta.author ?? ""}
          onChange={(e) =>
            edit((d) => {
              if (e.target.value) d.meta.author = e.target.value;
              else delete d.meta.author;
            })
          }
        />
      </Row>
    </>
  );
}

const PAPERS: { value: Paper["kind"]; label: string }[] = [
  { value: "a4", label: "A4 (210 × 297 mm)" },
  { value: "letter", label: "US Letter (8.5 × 11 in)" },
  { value: "a5", label: "A5 (148 × 210 mm)" },
  { value: "legal", label: "US Legal (8.5 × 14 in)" },
  { value: "custom", label: "Custom size" },
];

const SPACINGS: { value: LineSpacing["kind"]; label: string }[] = [
  { value: "single", label: "Single" },
  { value: "oneHalf", label: "1.5 lines" },
  { value: "double", label: "Double" },
  { value: "custom", label: "Custom" },
];

export function PageSection({ t, edit, issues }: SectionProps) {
  const p = t.page;
  const twoSided = p.twoSided;
  return (
    <>
      <Row label="Paper" issues={issuesAt(issues, "page.paper")}>
        <Select
          value={p.paper.kind}
          options={PAPERS}
          onChange={(kind) =>
            edit((d) => {
              d.page.paper = kind === "custom" ? { kind, widthMm: 210, heightMm: 297 } : ({ kind } as Paper);
            })
          }
        />
        {p.paper.kind === "custom" && (
          <span className="d-inline">
            <NumberInput
              value={p.paper.widthMm}
              unit="mm wide"
              onChange={(v) => edit((d) => void (d.page.paper.kind === "custom" && (d.page.paper.widthMm = v)))}
            />
            <NumberInput
              value={p.paper.heightMm}
              unit="mm high"
              onChange={(v) => edit((d) => void (d.page.paper.kind === "custom" && (d.page.paper.heightMm = v)))}
            />
          </span>
        )}
      </Row>
      <Row label="Printing">
        <Toggle
          checked={twoSided}
          onChange={(v) => edit((d) => void (d.page.twoSided = v))}
          label="Two-sided (margins mirror on even pages)"
        />
      </Row>
      <Row label="Margins" hint="Paper edge to text" issues={issuesAt(issues, "page.margins")}>
        <div className="d-margins">
          <label>
            Top <NumberInput value={p.margins.topMm} unit="mm" step={0.5} onChange={(v) => edit((d) => void (d.page.margins.topMm = v))} />
          </label>
          <label>
            Bottom{" "}
            <NumberInput value={p.margins.bottomMm} unit="mm" step={0.5} onChange={(v) => edit((d) => void (d.page.margins.bottomMm = v))} />
          </label>
          <label>
            {twoSided ? "Inner" : "Left"}{" "}
            <NumberInput value={p.margins.innerMm} unit="mm" step={0.5} onChange={(v) => edit((d) => void (d.page.margins.innerMm = v))} />
          </label>
          <label>
            {twoSided ? "Outer" : "Right"}{" "}
            <NumberInput value={p.margins.outerMm} unit="mm" step={0.5} onChange={(v) => edit((d) => void (d.page.margins.outerMm = v))} />
          </label>
        </div>
      </Row>
      <Row label="Binding offset" hint="Extra space on the bound edge" issues={issuesAt(issues, "page.bindingOffsetMm")}>
        <NumberInput value={p.bindingOffsetMm} unit="mm" step={0.5} min={0} onChange={(v) => edit((d) => void (d.page.bindingOffsetMm = v))} />
      </Row>
      <Row label="Line spacing" issues={issuesAt(issues, "page.lineSpacing")}>
        <Select
          value={p.lineSpacing.kind}
          options={SPACINGS}
          onChange={(kind) =>
            edit((d) => {
              d.page.lineSpacing = kind === "custom" ? { kind, stretch: 1.25 } : ({ kind } as LineSpacing);
            })
          }
        />
        {p.lineSpacing.kind === "custom" && (
          <NumberInput
            value={p.lineSpacing.stretch}
            unit="× normal"
            step={0.05}
            min={0.5}
            onChange={(v) => edit((d) => void (d.page.lineSpacing.kind === "custom" && (d.page.lineSpacing.stretch = v)))}
          />
        )}
      </Row>
      <Row label="Paragraphs" issues={issuesAt(issues, "page.paragraph")}>
        <div className="d-margins">
          <label>
            First-line indent{" "}
            <NumberInput value={p.paragraph.indentMm} unit="mm" step={0.5} min={0} onChange={(v) => edit((d) => void (d.page.paragraph.indentMm = v))} />
          </label>
          <label>
            Space between{" "}
            <NumberInput value={p.paragraph.spacingPt} unit="pt" min={0} onChange={(v) => edit((d) => void (d.page.paragraph.spacingPt = v))} />
          </label>
        </div>
      </Row>
    </>
  );
}

const BUILTIN_FONTS: { value: BuiltinFont; label: string }[] = [
  { value: "latinModern", label: "Latin Modern (LaTeX default)" },
  { value: "termes", label: "TeX Gyre Termes (Times-like)" },
  { value: "pagella", label: "TeX Gyre Pagella (Palatino-like)" },
  { value: "bonum", label: "TeX Gyre Bonum (Bookman-like)" },
  { value: "schola", label: "TeX Gyre Schola (Century Schoolbook-like)" },
  { value: "heros", label: "TeX Gyre Heros (Helvetica-like)" },
  { value: "adventor", label: "TeX Gyre Adventor (Avant Garde-like)" },
  { value: "cursor", label: "TeX Gyre Cursor (Courier-like)" },
];

function FontPicker({ value, onChange, systemFonts }: { value: FontChoice; onChange: (f: FontChoice) => void; systemFonts: string[] }) {
  const selectValue = value.kind === "builtin" ? value.font : "__system";
  return (
    <span className="d-inline">
      <select
        value={selectValue}
        onChange={(e) =>
          onChange(
            e.target.value === "__system"
              ? { kind: "system", family: systemFonts[0] ?? "" }
              : { kind: "builtin", font: e.target.value as BuiltinFont },
          )
        }
      >
        <optgroup label="Always available">
          {BUILTIN_FONTS.map((f) => (
            <option key={f.value} value={f.value}>
              {f.label}
            </option>
          ))}
        </optgroup>
        <option value="__system">Installed on this computer…</option>
      </select>
      {value.kind === "system" && (
        <>
          <input list="system-fonts" value={value.family} placeholder="Font family" onChange={(e) => onChange({ kind: "system", family: e.target.value })} />
          <datalist id="system-fonts">
            {systemFonts.map((f) => (
              <option key={f} value={f} />
            ))}
          </datalist>
        </>
      )}
    </span>
  );
}

export function FontsSection({ t, edit, issues, systemFonts }: SectionProps & { systemFonts: string[] }) {
  const anySystem = [t.fonts.main, t.fonts.sans, t.fonts.mono].some((f) => f.kind === "system");
  return (
    <>
      <Row label="Body text size" issues={issuesAt(issues, "fonts.baseSizePt")}>
        <Select
          value={t.fonts.baseSizePt}
          options={[10, 11, 12].map((v) => ({ value: v, label: `${v} pt` }))}
          onChange={(v) => edit((d) => void (d.fonts.baseSizePt = v))}
        />
      </Row>
      <Row label="Main font" hint="Body text and headings" issues={issuesAt(issues, "fonts.main")}>
        <FontPicker value={t.fonts.main} systemFonts={systemFonts} onChange={(f) => edit((d) => void (d.fonts.main = f))} />
      </Row>
      <Row label="Sans-serif font" issues={issuesAt(issues, "fonts.sans")}>
        <FontPicker value={t.fonts.sans} systemFonts={systemFonts} onChange={(f) => edit((d) => void (d.fonts.sans = f))} />
      </Row>
      <Row label="Monospace font" hint="Code" issues={issuesAt(issues, "fonts.mono")}>
        <FontPicker value={t.fonts.mono} systemFonts={systemFonts} onChange={(f) => edit((d) => void (d.fonts.mono = f))} />
      </Row>
      {anySystem && (
        <p className="d-note">
          Fonts installed on this computer may be missing on other machines. The “always available” fonts come with
          the TeX engine and work everywhere.
        </p>
      )}
    </>
  );
}

const PAGE_NUMBERS: { value: PageNumberStyle; label: string }[] = [
  { value: "arabic", label: "1, 2, 3" },
  { value: "lowerRoman", label: "i, ii, iii" },
  { value: "upperRoman", label: "I, II, III" },
];

export function StructureSection({ t, edit, issues }: SectionProps) {
  const s = t.structure;
  const depthOptions = (names: string[]) =>
    [0, 1, 2, 3, 4].map((n) => ({ value: n, label: n === 0 ? "None" : `Down to ${names[n - 1].toLowerCase()}s` }));
  const names = s.topLevel === "chapter" ? ["Chapter", "Section", "Subsection", "Subsubsection"] : ["Section", "Subsection", "Subsubsection", "Paragraph"];
  return (
    <>
      <Row label="Top-level heading" hint="What heading level 1 becomes">
        <Select
          value={s.topLevel}
          options={[
            { value: "chapter", label: "Chapters (each starts a new page)" },
            { value: "section", label: "Sections (shorter documents)" },
          ]}
          onChange={(v) => edit((d) => void (d.structure.topLevel = v))}
        />
      </Row>
      <Row label="Numbered headings" issues={issuesAt(issues, "structure.numberedDepth")}>
        <Select value={s.numberedDepth} options={depthOptions(names)} onChange={(v) => edit((d) => void (d.structure.numberedDepth = v))} />
      </Row>
      <Row label="In table of contents" issues={issuesAt(issues, "structure.tocDepth")}>
        <Select value={s.tocDepth} options={depthOptions(names)} onChange={(v) => edit((d) => void (d.structure.tocDepth = v))} />
      </Row>
      <Row label="Page numbers" hint="Front matter / main text">
        <span className="d-inline">
          <Select
            value={s.frontMatterPageNumbers}
            options={PAGE_NUMBERS}
            onChange={(v) => edit((d) => void (d.structure.frontMatterPageNumbers = v))}
          />
          <span className="muted">then</span>
          <Select value={s.mainPageNumbers} options={PAGE_NUMBERS} onChange={(v) => edit((d) => void (d.structure.mainPageNumbers = v))} />
        </span>
      </Row>
      {s.topLevel === "chapter" && (
        <Row label="Equations, figures, tables">
          <Select
            value={s.perChapterNumbering ? "chapter" : "continuous"}
            options={[
              { value: "chapter", label: "Numbered per chapter (2.1, 2.2…)" },
              { value: "continuous", label: "Numbered continuously (1, 2, 3…)" },
            ]}
            onChange={(v) => edit((d) => void (d.structure.perChapterNumbering = v === "chapter"))}
          />
        </Row>
      )}
    </>
  );
}
