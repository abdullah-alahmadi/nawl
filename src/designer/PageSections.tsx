import { useEffect, useState } from "react";
// Designer panels: template info, page, fonts, structure.
import type { BuiltinFont } from "../bindings/BuiltinFont";
import type { Language } from "../bindings/Language";
import type { LanguageSettings } from "../bindings/LanguageSettings";
import type { FontChoice } from "../bindings/FontChoice";
import type { Paper } from "../bindings/Paper";
import type { LineSpacing } from "../bindings/LineSpacing";
import type { PageNumberStyle } from "../bindings/PageNumberStyle";
import { NumberInput, Row, Segmented, Select, Toggle, issuesAt } from "./controls";
import { headingLevels, type SectionProps, type Translate } from "./common";
import { useT, type MessageKey } from "../i18n";
import * as api from "../api";
import { arabicSystemFonts, ensureLibraryFont, libraryFamily, libraryFonts } from "../editor/libraryFonts";

export function InfoSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  return (
    <>
      <Row label={tr("common.name")} issues={issuesAt(issues, "meta.name")}>
        <input value={t.meta.name} onChange={(e) => edit((d) => void (d.meta.name = e.target.value))} />
      </Row>
      <Row label={tr("designer.id")} hint={tr("designer.idHint")} issues={issuesAt(issues, "meta.id")}>
        <input
          className="mono"
          value={t.meta.id}
          onChange={(e) => edit((d) => void (d.meta.id = e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, "-")))}
        />
      </Row>
      <Row label={tr("designer.description")}>
        <textarea rows={3} value={t.meta.description} onChange={(e) => edit((d) => void (d.meta.description = e.target.value))} />
      </Row>
      <Row label={tr("designer.version")}>
        <input value={t.meta.version} onChange={(e) => edit((d) => void (d.meta.version = e.target.value))} />
      </Row>
      <Row label={tr("designer.author")}>
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

// Paper names and sizes stay as they are in every language.
const papers = (tr: Translate): { value: Paper["kind"]; label: string }[] => [
  { value: "a4", label: "A4 (210 × 297 mm)" },
  { value: "letter", label: "US Letter (8.5 × 11 in)" },
  { value: "a5", label: "A5 (148 × 210 mm)" },
  { value: "legal", label: "US Legal (8.5 × 14 in)" },
  { value: "custom", label: tr("designer.paperCustom") },
];

const spacings = (tr: Translate): { value: LineSpacing["kind"]; label: string }[] => [
  { value: "single", label: tr("designer.spacingSingle") },
  { value: "oneHalf", label: tr("designer.spacingOneHalf") },
  { value: "double", label: tr("designer.spacingDouble") },
  { value: "custom", label: tr("designer.custom") },
];

export function PageSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const p = t.page;
  const twoSided = p.twoSided;
  return (
    <>
      <Row label={tr("designer.paper")} issues={issuesAt(issues, "page.paper")}>
        <Select
          value={p.paper.kind}
          options={papers(tr)}
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
              unit={tr("designer.mmWide")}
              onChange={(v) => edit((d) => void (d.page.paper.kind === "custom" && (d.page.paper.widthMm = v)))}
            />
            <NumberInput
              value={p.paper.heightMm}
              unit={tr("designer.mmHigh")}
              onChange={(v) => edit((d) => void (d.page.paper.kind === "custom" && (d.page.paper.heightMm = v)))}
            />
          </span>
        )}
      </Row>
      <Row label={tr("designer.printing")}>
        <Toggle
          checked={twoSided}
          onChange={(v) => edit((d) => void (d.page.twoSided = v))}
          label={tr("designer.twoSided")}
        />
      </Row>
      <Row label={tr("designer.columns")} hint={tr("designer.columnsHint")} issues={issuesAt(issues, "page.columns")}>
        <Segmented
          value={String(p.columns ?? 1) as "1" | "2"}
          options={[
            { value: "1", label: tr("designer.oneColumn") },
            { value: "2", label: tr("designer.twoColumns") },
          ]}
          onChange={(v) =>
            edit((d) => {
              if (v === "2") d.page.columns = 2;
              else delete d.page.columns;
            })
          }
        />
      </Row>
      <Row label={tr("designer.margins")} hint={tr("designer.marginsHint")} issues={issuesAt(issues, "page.margins")}>
        <div className="d-margins">
          <label>
            {tr("common.top")} <NumberInput value={p.margins.topMm} unit="mm" step={0.5} onChange={(v) => edit((d) => void (d.page.margins.topMm = v))} />
          </label>
          <label>
            {tr("common.bottom")}{" "}
            <NumberInput value={p.margins.bottomMm} unit="mm" step={0.5} onChange={(v) => edit((d) => void (d.page.margins.bottomMm = v))} />
          </label>
          <label>
            {tr(twoSided ? "designer.inner" : "common.left")}{" "}
            <NumberInput value={p.margins.innerMm} unit="mm" step={0.5} onChange={(v) => edit((d) => void (d.page.margins.innerMm = v))} />
          </label>
          <label>
            {tr(twoSided ? "designer.outer" : "common.right")}{" "}
            <NumberInput value={p.margins.outerMm} unit="mm" step={0.5} onChange={(v) => edit((d) => void (d.page.margins.outerMm = v))} />
          </label>
        </div>
      </Row>
      <Row label={tr("designer.bindingOffset")} hint={tr("designer.bindingOffsetHint")} issues={issuesAt(issues, "page.bindingOffsetMm")}>
        <NumberInput value={p.bindingOffsetMm} unit="mm" step={0.5} min={0} onChange={(v) => edit((d) => void (d.page.bindingOffsetMm = v))} />
      </Row>
      <Row label={tr("designer.lineSpacing")} issues={issuesAt(issues, "page.lineSpacing")}>
        <Select
          value={p.lineSpacing.kind}
          options={spacings(tr)}
          onChange={(kind) =>
            edit((d) => {
              d.page.lineSpacing = kind === "custom" ? { kind, stretch: 1.25 } : ({ kind } as LineSpacing);
            })
          }
        />
        {p.lineSpacing.kind === "custom" && (
          <NumberInput
            value={p.lineSpacing.stretch}
            unit={tr("designer.timesNormal")}
            step={0.05}
            min={0.5}
            onChange={(v) => edit((d) => void (d.page.lineSpacing.kind === "custom" && (d.page.lineSpacing.stretch = v)))}
          />
        )}
      </Row>
      <Row label={tr("designer.paragraphs")} issues={issuesAt(issues, "page.paragraph")}>
        <div className="d-margins">
          <label>
            {tr("designer.firstLineIndent")}{" "}
            <NumberInput value={p.paragraph.indentMm} unit="mm" step={0.5} min={0} onChange={(v) => edit((d) => void (d.page.paragraph.indentMm = v))} />
          </label>
          <label>
            {tr("designer.spaceBetween")}{" "}
            <NumberInput value={p.paragraph.spacingPt} unit="pt" min={0} onChange={(v) => edit((d) => void (d.page.paragraph.spacingPt = v))} />
          </label>
        </div>
      </Row>
    </>
  );
}

// Labels are message keys (the font names stay, the notes after them are translated).
const BUILTIN_FONTS: { value: BuiltinFont; label: MessageKey }[] = [
  { value: "latinModern", label: "designer.fontLatinModern" },
  { value: "termes", label: "designer.fontTermes" },
  { value: "pagella", label: "designer.fontPagella" },
  { value: "bonum", label: "designer.fontBonum" },
  { value: "schola", label: "designer.fontSchola" },
  { value: "heros", label: "designer.fontHeros" },
  { value: "adventor", label: "designer.fontAdventor" },
  { value: "cursor", label: "designer.fontCursor" },
  { value: "amiri", label: "designer.fontAmiri" },
];

/** A font: TeX's own, one from Texloom's library, or an installed one. With
 *  `inheritLabel`, also "none of its own" (`undefined`: a heading takes the text's). */
export function FontPicker({
  value,
  onChange,
  systemFonts,
  inheritLabel,
}: {
  value: FontChoice | undefined;
  onChange: (f: FontChoice | undefined) => void;
  systemFonts: string[];
  inheritLabel?: string;
}) {
  const tr = useT();
  const [library, setLibrary] = useState<api.LibraryFont[]>([]);
  const [arabic, setArabic] = useState<string[]>([]);
  useEffect(() => {
    void libraryFonts().then((list) => setLibrary(list.filter((f) => f.available)));
    void arabicSystemFonts().then(setArabic);
  }, []);
  // Installed fonts, picked from the list: those with Arabic letters first (Diwani,
  // Thuluth…), then the others. A name in neither (a template from another computer)
  // keeps an entry of its own.
  const arabicSet = new Set(arabic);
  const others = systemFonts.filter((f) => !arabicSet.has(f));
  const family = value?.kind === "system" ? value.family : null;
  const unlisted = family !== null && !arabicSet.has(family) && !others.includes(family);
  const selectValue = !value ? "__inherit" : value.kind === "builtin" ? value.font : value.kind === "library" ? `lib:${value.id}` : `sys:${value.family}`;
  return (
    <span className="d-inline">
      <select
        value={selectValue}
        onChange={(e) => {
          const v = e.target.value;
          if (v === "__inherit") onChange(undefined);
          else if (v.startsWith("sys:")) onChange({ kind: "system", family: v.slice(4) });
          else if (v.startsWith("lib:")) {
            void ensureLibraryFont(v.slice(4));
            onChange({ kind: "library", id: v.slice(4) });
          } else onChange({ kind: "builtin", font: v as BuiltinFont });
        }}
      >
        {inheritLabel && <option value="__inherit">{inheritLabel}</option>}
        <optgroup label={tr("designer.fontsAlwaysAvailable")}>
          {BUILTIN_FONTS.map((f) => (
            <option key={f.value} value={f.value}>
              {tr(f.label)}
            </option>
          ))}
        </optgroup>
        {FONT_CATEGORIES.map(({ key, label }) => {
          const list = library.filter((f) => f.category === key);
          return list.length === 0 ? null : (
            <optgroup key={key} label={`${tr("designer.fontLibrary")}: ${tr(label)}`}>
              {list.map((f) => (
                <option key={f.id} value={`lib:${f.id}`}>
                  {f.name}
                </option>
              ))}
            </optgroup>
          );
        })}
        {unlisted && family && <option value={`sys:${family}`}>{tr("designer.fontNotInstalled", { name: family })}</option>}
        {arabic.length > 0 && (
          <optgroup label={tr("designer.fontInstalledArabicGroup")}>
            {arabic.map((f) => (
              <option key={f} value={`sys:${f}`}>
                {f}
              </option>
            ))}
          </optgroup>
        )}
        {others.length > 0 && (
          <optgroup label={tr("designer.fontInstalledOtherGroup")}>
            {others.map((f) => (
              <option key={f} value={`sys:${f}`}>
                {f}
              </option>
            ))}
          </optgroup>
        )}
      </select>
    </span>
  );
}

export function FontsSection({ t, edit, issues, systemFonts }: SectionProps & { systemFonts: string[] }) {
  const tr = useT();
  const anySystem = [t.fonts.main, t.fonts.sans, t.fonts.mono].some((f) => f.kind === "system");
  // On a publisher's class or a beamer theme, the class or theme sets the text size and
  // the Latin fonts; the Arabic font always applies, and in an Arabic document so do
  // the English fonts (polyglossia sets both up after the class).
  const ownLayout = !t.base && t.kind !== "presentation";
  const arabicMain = t.language?.main === "arabic";
  const latinFonts = ownLayout || arabicMain;
  return (
    <>
      {ownLayout && (
        <Row label={tr("designer.bodyTextSize")} issues={issuesAt(issues, "fonts.baseSizePt")}>
          <Select
            value={t.fonts.baseSizePt}
            options={[10, 11, 12].map((v) => ({ value: v, label: `${v} pt` }))}
            onChange={(v) => edit((d) => void (d.fonts.baseSizePt = v))}
          />
        </Row>
      )}
      {!ownLayout && <p className="d-note">{tr(latinFonts ? "designer.fontsOnClassArabic" : "designer.fontsOnClass")}</p>}
      {latinFonts && (
        <>
          <Row label={tr(ownLayout ? "designer.mainFont" : "designer.englishMainFont")} hint={tr("designer.mainFontHint")} issues={issuesAt(issues, "fonts.main")}>
            <FontPicker value={t.fonts.main} systemFonts={systemFonts} onChange={(f) => f && edit((d) => void (d.fonts.main = f))} />
          </Row>
          <Row label={tr("designer.sansFont")} issues={issuesAt(issues, "fonts.sans")}>
            <FontPicker value={t.fonts.sans} systemFonts={systemFonts} onChange={(f) => f && edit((d) => void (d.fonts.sans = f))} />
          </Row>
          <Row label={tr("designer.monoFont")} hint={tr("designer.monoFontHint")} issues={issuesAt(issues, "fonts.mono")}>
            <FontPicker value={t.fonts.mono} systemFonts={systemFonts} onChange={(f) => f && edit((d) => void (d.fonts.mono = f))} />
          </Row>
        </>
      )}
      <Row label={tr("designer.arabicFont")} hint={tr("designer.arabicFontHint")} issues={issuesAt(issues, "fonts.arabic")}>
        <ArabicFontGallery
          value={t.fonts.arabic}
          systemFonts={systemFonts}
          onChange={(f) =>
            edit((d) => {
              if (f) d.fonts.arabic = f;
              else delete d.fonts.arabic;
            })
          }
        />
      </Row>
      {anySystem && (
        <p className="d-note">{tr("designer.systemFontsNote")}</p>
      )}
    </>
  );
}

const FONT_CATEGORIES: { key: string; label: MessageKey }[] = [
  { key: "naskh", label: "designer.fontCategoryNaskh" },
  { key: "sans", label: "designer.fontCategorySans" },
  { key: "kufi", label: "designer.fontCategoryKufi" },
  { key: "calligraphic", label: "designer.fontCategoryCalligraphic" },
  { key: "display", label: "designer.fontCategoryDisplay" },
  { key: "nastaliq", label: "designer.fontCategoryNastaliq" },
];

/** A line of Arabic that shows a font's character: letters of every shape, digits. */
const FONT_SAMPLE = "أبجد هوز حطي كلمن ١٢٣";

/** The Arabic font: Amiri (built in), a family from Texloom's font library (each shown
 *  in its own face, by style), or a font installed on this computer. */
function ArabicFontGallery({
  value,
  systemFonts,
  onChange,
}: {
  value: FontChoice | undefined;
  systemFonts: string[];
  onChange: (f: FontChoice | undefined) => void;
}) {
  const tr = useT();
  const [fonts, setFonts] = useState<api.LibraryFont[]>([]);
  const [installed, setInstalled] = useState<string[]>([]);
  useEffect(() => {
    api.listArabicSystemFonts().then(setInstalled, () => setInstalled([]));
  }, []);
  useEffect(() => {
    api.listLibraryFonts().then(
      (list) => {
        setFonts(list);
        // Each card shows its font, so they're all loaded (about 6 MB, once).
        for (const f of list) if (f.available) void ensureLibraryFont(f.id);
      },
      () => setFonts([]),
    );
  }, []);
  const card = (selected: boolean) => `font-card${selected ? " selected" : ""}`;
  return (
    <div className="font-gallery">
      <div className="font-grid">
        <button type="button" className={card(!value || value.kind === "builtin")} onClick={() => onChange(undefined)} aria-pressed={!value || value.kind === "builtin"}>
          <span className="font-card-name">Amiri · {tr("designer.fontBuiltIn")}</span>
          <span className="font-card-sample" style={{ fontFamily: '"Amiri", "Traditional Arabic", serif' }} dir="rtl" lang="ar">
            {FONT_SAMPLE}
          </span>
        </button>
      </div>
      {FONT_CATEGORIES.map(({ key, label }) => {
        const list = fonts.filter((f) => f.category === key && f.available);
        if (list.length === 0) return null;
        return (
          <section key={key}>
            <h4 className="font-category">{tr(label)}</h4>
            <div className="font-grid">
              {list.map((f) => {
                const selected = value?.kind === "library" && value.id === f.id;
                return (
                  <button key={f.id} type="button" className={card(selected)} aria-pressed={selected} onClick={() => onChange({ kind: "library", id: f.id })}>
                    <span className="font-card-name">{f.name}</span>
                    <span className="font-card-sample" style={{ fontFamily: `"${libraryFamily(f.id)}", serif` }} dir="rtl" lang="ar">
                      {FONT_SAMPLE}
                    </span>
                  </button>
                );
              })}
            </div>
          </section>
        );
      })}
      {(() => {
        // Installed fonts with Arabic letters, each shown in its own face (the
        // library's and Amiri, already above, left out).
        const shown = new Set([...fonts.map((f) => f.name.toLowerCase()), "amiri"]);
        const list = installed.filter((f) => !shown.has(f.toLowerCase()));
        if (list.length === 0) return null;
        return (
          <section>
            <h4 className="font-category">{tr("designer.fontInstalledArabic")}</h4>
            <p className="d-note">{tr("designer.fontInstalledArabicNote")}</p>
            <div className="font-grid">
              {list.map((f) => {
                const selected = value?.kind === "system" && value.family === f;
                return (
                  <button key={f} type="button" className={card(selected)} aria-pressed={selected} onClick={() => onChange({ kind: "system", family: f })}>
                    <span className="font-card-name">{f}</span>
                    <span className="font-card-sample" style={{ fontFamily: `"${f}", serif` }} dir="rtl" lang="ar">
                      {FONT_SAMPLE}
                    </span>
                  </button>
                );
              })}
            </div>
          </section>
        );
      })()}
      <h4 className="font-category">{tr("designer.fontInstalled")}</h4>
      <span className="d-inline">
        <input
          list="system-fonts-arabic"
          value={value?.kind === "system" ? value.family : ""}
          placeholder={tr("designer.arabicFontPlaceholder")}
          onChange={(e) => onChange(e.target.value.trim() ? { kind: "system", family: e.target.value } : undefined)}
        />
        <datalist id="system-fonts-arabic">
          {systemFonts.map((f) => (
            <option key={f} value={f} />
          ))}
        </datalist>
      </span>
      <p className="d-note">{tr("designer.fontLibraryNote")}</p>
    </div>
  );
}

const LANGUAGES: { value: Language; label: MessageKey }[] = [
  { value: "english", label: "designer.langEnglish" },
  { value: "arabic", label: "designer.langArabic" },
];

/** The document's language, a second language for passages in it, and digits. */
export function LanguageSection({ t, edit, systemFonts }: SectionProps & { systemFonts: string[] }) {
  const tr = useT();
  const languages = LANGUAGES.map((l) => ({ value: l.value, label: tr(l.label) }));
  const lang: LanguageSettings = t.language ?? { main: "english", other: null, digits: "western", math: "international" };
  const set = (patch: Partial<LanguageSettings>) =>
    edit((d) => {
      const next = { ...lang, ...patch };
      // English only, the default: no setting at all (as in older templates).
      if (next.main === "english" && !next.other && next.digits === "western" && next.math === "international") delete d.language;
      else d.language = next;
    });
  const arabic = lang.main === "arabic" || lang.other === "arabic";
  return (
    <>
      <Row label={tr("designer.language")} hint={tr("designer.languageHint")}>
        <Select value={lang.main} options={languages} onChange={(v) => set({ main: v, other: lang.other === v ? null : lang.other })} />
      </Row>
      <Row label={tr("designer.secondLanguage")} hint={tr("designer.secondLanguageHint")}>
        <Select
          value={lang.other ?? ""}
          options={[{ value: "", label: tr("common.none") }, ...languages.filter((l) => l.value !== lang.main)]}
          onChange={(v) => set({ other: (v || null) as Language | null })}
        />
      </Row>
      {arabic && (
        <Row label={tr("designer.digits")} hint={tr("designer.digitsHint")}>
          <Segmented
            value={lang.digits}
            options={[
              { value: "western", label: "1 2 3" },
              { value: "arabicIndic", label: "١ ٢ ٣" },
            ]}
            onChange={(v) => set({ digits: v })}
          />
        </Row>
      )}
      {lang.main === "arabic" && (
        <Row label={tr("designer.mathStyle")} hint={tr("designer.mathStyleHint")}>
          <Segmented
            value={lang.math ?? "international"}
            options={[
              { value: "international", label: tr("designer.mathInternational") },
              { value: "arabic", label: tr("designer.mathArabic") },
            ]}
            onChange={(v) => set({ math: v })}
          />
        </Row>
      )}
      {lang.main === "arabic" && lang.math === "arabic" && (
        <Row label={tr("designer.mathFont")} hint={tr("designer.mathFontHint")}>
          <ArabicFontGallery
            value={t.fonts.arabicMath ?? { kind: "library", id: "aref-ruqaa" }}
            systemFonts={systemFonts}
            onChange={(f) =>
              edit((d) => {
                // Aref Ruqaa is the default: no setting at all.
                if (f && !(f.kind === "library" && f.id === "aref-ruqaa")) d.fonts.arabicMath = f;
                else delete d.fonts.arabicMath;
              })
            }
          />
        </Row>
      )}
      <p className="d-note">{tr("designer.languageNote")}</p>
    </>
  );
}

const PAGE_NUMBERS: { value: PageNumberStyle; label: string }[] = [
  { value: "arabic", label: "1, 2, 3" },
  { value: "lowerRoman", label: "i, ii, iii" },
  { value: "upperRoman", label: "I, II, III" },
];

export function StructureSection({ t, edit, issues }: SectionProps) {
  const tr = useT();
  const s = t.structure;
  const levels = headingLevels(t);
  const depthOptions = [0, 1, 2, 3, 4].map((n) => ({
    value: n,
    label: n === 0 ? tr("common.none") : tr(`designer.depth${levels[n - 1]}`),
  }));
  return (
    <>
      <Row label={tr("designer.topLevel")} hint={tr("designer.topLevelHint")}>
        <Select
          value={s.topLevel}
          options={[
            { value: "chapter", label: tr("designer.topChapters") },
            { value: "section", label: tr("designer.topSections") },
          ]}
          onChange={(v) => edit((d) => void (d.structure.topLevel = v))}
        />
      </Row>
      <Row label={tr("designer.numberedHeadings")} issues={issuesAt(issues, "structure.numberedDepth")}>
        <Select value={s.numberedDepth} options={depthOptions} onChange={(v) => edit((d) => void (d.structure.numberedDepth = v))} />
      </Row>
      <Row label={tr("designer.inToc")} issues={issuesAt(issues, "structure.tocDepth")}>
        <Select value={s.tocDepth} options={depthOptions} onChange={(v) => edit((d) => void (d.structure.tocDepth = v))} />
      </Row>
      <Row label={tr("designer.pageNumbers")} hint={tr("designer.pageNumbersHint")}>
        <span className="d-inline">
          <Select
            value={s.frontMatterPageNumbers}
            options={PAGE_NUMBERS}
            onChange={(v) => edit((d) => void (d.structure.frontMatterPageNumbers = v))}
          />
          <span className="muted">{tr("designer.then")}</span>
          <Select value={s.mainPageNumbers} options={PAGE_NUMBERS} onChange={(v) => edit((d) => void (d.structure.mainPageNumbers = v))} />
        </span>
      </Row>
      <Row label={tr("designer.appendixLabel")} hint={tr("designer.appendixLabelHint")} issues={issuesAt(issues, "structure.appendixLabel")}>
        <input value={s.appendixLabel} onChange={(e) => edit((d) => void (d.structure.appendixLabel = e.target.value))} />
      </Row>
      {s.topLevel === "chapter" && (
        <Row label={tr("designer.equationsFiguresTables")}>
          <Select
            value={s.perChapterNumbering ? "chapter" : "continuous"}
            options={[
              { value: "chapter", label: tr("designer.perChapter") },
              { value: "continuous", label: tr("designer.continuous") },
            ]}
            onChange={(v) => edit((d) => void (d.structure.perChapterNumbering = v === "chapter"))}
          />
        </Row>
      )}
      {s.topLevel === "chapter" && (
        <Row label={tr("designer.coverPages")} hint={tr("designer.coverPagesHint")}>
          <Toggle
            checked={!!s.coverPages}
            onChange={(on) => edit((d) => void (on ? (d.structure.coverPages = true) : delete d.structure.coverPages))}
            label={tr("designer.coverPagesLabel")}
          />
        </Row>
      )}
    </>
  );
}
