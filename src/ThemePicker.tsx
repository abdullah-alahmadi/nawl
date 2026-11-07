// The theme picker for presentations: every beamer theme (and colour theme) as a
// preview rendered by the real theme, Texloom's own list plus whatever the installed
// TeX has, and a main colour of the user's own. Choosing restyles the deck at once.
import { useEffect, useMemo, useRef, useState } from "react";
import * as pdfjs from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { Check, LoaderCircle, Palette, Pipette, Search, X } from "lucide-react";
import * as api from "./api";
import type { PresentationSettings } from "./bindings/PresentationSettings";
import { BEAMER_COLOR_THEMES, BEAMER_THEMES } from "./editor/templateStyle";
import { useT } from "./i18n";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

/** Previews by key, shared while the app runs (the backend caches them on disk too). */
const previews = new Map<string, Promise<ArrayBuffer>>();
/** At most a few renders at a time, in the order the cards appear. */
const queue: (() => void)[] = [];
let running = 0;
const MAX_RENDERS = 3;

function schedule<T>(job: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const start = () => {
      running++;
      job()
        .then(resolve, reject)
        .finally(() => {
          running--;
          queue.shift()?.();
        });
    };
    if (running < MAX_RENDERS) start();
    else queue.push(start);
  });
}

function preview(theme: string, colorTheme: string, fontTheme: string, installed: boolean): Promise<ArrayBuffer> {
  const key = `${theme}|${colorTheme}|${fontTheme}`;
  let p = previews.get(key);
  if (!p) {
    p = schedule(() => api.themePreview(theme, colorTheme, fontTheme, installed));
    // A failure isn't cached: the next opening tries again.
    p.catch(() => previews.delete(key));
    previews.set(key, p);
  }
  return p;
}

/** The title slide and the content slide of a preview PDF. */
function Thumb({ theme, colorTheme, fontTheme, installed }: { theme: string; colorTheme: string; fontTheme: string; installed: boolean }) {
  const t = useT();
  const box = useRef<HTMLDivElement>(null);
  const [state, setState] = useState<"waiting" | "loading" | "done" | "failed">("waiting");

  useEffect(() => {
    const el = box.current;
    if (!el) return;
    let cancelled = false;
    // Render only once the card scrolls into view.
    const io = new IntersectionObserver((entries) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      io.disconnect();
      setState("loading");
      preview(theme, colorTheme, fontTheme, installed)
        .then(async (bytes) => {
          const task = pdfjs.getDocument({ data: new Uint8Array(bytes.slice(0)) });
          const doc = await task.promise;
          const dpr = window.devicePixelRatio || 1;
          const canvases: HTMLCanvasElement[] = [];
          for (let i = 1; i <= Math.min(2, doc.numPages); i++) {
            const page = await doc.getPage(i);
            const base = page.getViewport({ scale: 1 });
            const width = (el.clientWidth - 6) / 2;
            const viewport = page.getViewport({ scale: (width / base.width) * dpr });
            const canvas = document.createElement("canvas");
            canvas.width = Math.floor(viewport.width);
            canvas.height = Math.floor(viewport.height);
            canvas.style.width = `${Math.floor(viewport.width / dpr)}px`;
            await page.render({ canvas, canvasContext: canvas.getContext("2d")!, viewport }).promise;
            canvases.push(canvas);
          }
          await task.destroy();
          if (cancelled) return;
          el.replaceChildren(...canvases);
          setState("done");
        })
        .catch(() => !cancelled && setState("failed"));
    });
    io.observe(el);
    return () => {
      cancelled = true;
      io.disconnect();
    };
  }, [theme, colorTheme, fontTheme, installed]);

  return (
    <span className="theme-thumb-wrap">
      <span className="theme-thumb" ref={box} />
      {state !== "done" && (
        <span className={`theme-thumb-state${state === "failed" ? " failed" : ""}`}>
          {state === "failed" ? t("dialogs.noThemePreview") : <LoaderCircle size={16} className="spin" aria-label={t("dialogs.rendering")} />}
        </span>
      )}
    </span>
  );
}

/** Colours that suit slides: deep enough for white text on a title bar. */
const ACCENTS = ["#1f4e79", "#2e7d32", "#8e1b1b", "#6a1b9a", "#00695c", "#c75b00", "#37474f", "#0d47a1", "#ad1457", "#5d4037"];

export function ThemePicker({
  settings,
  engine,
  onChange,
  onClose,
}: {
  settings: PresentationSettings;
  /** The TeX engine from Settings (previews and slides compile with it). */
  engine: string;
  onChange: (patch: Partial<PresentationSettings>, installedOnly: boolean) => void;
  onClose: () => void;
}) {
  const t = useT();
  const [tab, setTab] = useState<"theme" | "colour">("theme");
  const [query, setQuery] = useState("");
  const [installed, setInstalled] = useState<api.BeamerThemeList>({ themes: [], colorThemes: [], fontThemes: [] });
  useEffect(() => {
    api.listBeamerThemes().then(setInstalled, () => {});
  }, []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  // Texloom's list first (they work with every engine), then the installed TeX's others.
  const merge = (own: string[], extra: string[]) => {
    const lower = new Set(own.map((n) => n.toLowerCase()));
    return [...own.map((name) => ({ name, installedOnly: false })), ...extra.filter((n) => !lower.has(n.toLowerCase())).map((name) => ({ name, installedOnly: true }))];
  };
  const themes = useMemo(() => merge(BEAMER_THEMES, installed.themes), [installed]);
  const colours = useMemo(() => merge(BEAMER_COLOR_THEMES.filter((c) => c !== "default"), installed.colorThemes.filter((c) => c !== "structure")), [installed]);
  const q = query.trim().toLowerCase();
  const list = (tab === "theme" ? themes : colours).filter((th) => !q || th.name.toLowerCase().includes(q));
  const themeInstalledOnly = themes.find((th) => th.name === settings.theme)?.installedOnly ?? false;

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal gallery theme-picker" role="dialog" aria-modal="true" aria-label={t("dialogs.presentationTheme")}>
        <div className="gallery-main">
          <div className="gallery-head">
            <h2>{t("dialogs.presentationTheme")}</h2>
            <span className="segmented theme-tabs" role="tablist">
              <button role="tab" aria-selected={tab === "theme"} className={tab === "theme" ? "active" : ""} onClick={() => setTab("theme")}>
                {t("dialogs.themeTab")}
              </button>
              <button role="tab" aria-selected={tab === "colour"} className={tab === "colour" ? "active" : ""} onClick={() => setTab("colour")}>
                <Palette size={14} aria-hidden="true" /> {t("dialogs.coloursTab")}
              </button>
            </span>
            <label className="gallery-search">
              <Search size={14} aria-hidden="true" />
              <input type="search" placeholder={tab === "theme" ? t("dialogs.searchCountThemes", { n: themes.length }) : t("dialogs.searchCountColourThemes", { n: colours.length })} value={query} onChange={(e) => setQuery(e.target.value)} spellCheck={false} />
            </label>
            <button className="icon-button" onClick={onClose} aria-label={t("common.close")} title={t("dialogs.closeEsc")}>
              <X size={17} />
            </button>
          </div>
          <div className="gallery-body">
            {tab === "colour" && (
              <section className="accent-row">
                <h3>{t("dialogs.mainColour")}</h3>
                <div className="accent-swatches">
                  <button className={`accent-none${settings.accentColor ? "" : " active"}`} onClick={() => onChange({ accentColor: undefined }, themeInstalledOnly)} title={t("dialogs.themesOwnColours")}>
                    {t("dialogs.themesOwn")}
                  </button>
                  {ACCENTS.map((c) => (
                    <button
                      key={c}
                      className={`swatch${settings.accentColor?.toLowerCase() === c ? " active" : ""}`}
                      style={{ background: c }}
                      title={c}
                      aria-label={t("dialogs.mainColourNamed", { color: c })}
                      onClick={() => onChange({ accentColor: c }, themeInstalledOnly)}
                    />
                  ))}
                  <label className={`swatch-custom${settings.accentColor && !ACCENTS.includes(settings.accentColor.toLowerCase()) ? " active" : ""}`} title={t("dialogs.customColour")} style={settings.accentColor && !ACCENTS.includes(settings.accentColor.toLowerCase()) ? { background: settings.accentColor } : undefined}>
                    <Pipette size={13} aria-hidden="true" />
                    <input type="color" value={settings.accentColor ?? "#1f4e79"} onChange={(e) => onChange({ accentColor: e.target.value }, themeInstalledOnly)} aria-label={t("dialogs.customMainColour")} />
                  </label>
                </div>
                <p className="muted gallery-note">{t(settings.accentColor ? "dialogs.accentNoteOverride" : "dialogs.accentNote")}</p>
              </section>
            )}
            <section>
              {tab === "colour" && <h3>{t("dialogs.colourThemesWith", { theme: settings.theme })}</h3>}
              <div className="gallery-grid theme-grid">
                {tab === "colour" && !q && (
                  <ThemeCard
                    label={t("dialogs.themesOwn")}
                    current={!settings.colorTheme}
                    onClick={() => onChange({ colorTheme: "" }, themeInstalledOnly)}
                    theme={settings.theme}
                    colorTheme=""
                    fontTheme={settings.fontTheme}
                    installedOnly={themeInstalledOnly}
                  />
                )}
                {list.map((th) => (
                  <ThemeCard
                    key={th.name}
                    label={th.name}
                    badge={th.installedOnly ? t("dialogs.sourceTex") : undefined}
                    current={tab === "theme" ? settings.theme === th.name : settings.colorTheme === th.name}
                    onClick={() => onChange(tab === "theme" ? { theme: th.name } : { colorTheme: th.name }, tab === "theme" ? th.installedOnly : th.installedOnly || themeInstalledOnly)}
                    theme={tab === "theme" ? th.name : settings.theme}
                    colorTheme={tab === "theme" ? "" : th.name}
                    fontTheme={settings.fontTheme}
                    installedOnly={th.installedOnly || (tab === "colour" && themeInstalledOnly)}
                  />
                ))}
              </div>
              {list.length === 0 && <p className="muted gallery-empty">{t("dialogs.nothingMatches", { query: query.trim() })}</p>}
            </section>
          </div>
          <div className="gallery-foot">
            <span className="muted">
              {engine === "tectonic" && installed.themes.length > 0
                ? t("dialogs.themePreviewsNoteInstalled")
                : t("dialogs.themePreviewsNote", { engine: engine === "tectonic" ? "Tectonic" : engine })}
            </span>
            <button className="tb-button" onClick={onClose}>
              <Check size={15} aria-hidden="true" /> {t("common.done")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function ThemeCard({
  label,
  badge,
  current,
  onClick,
  theme,
  colorTheme,
  fontTheme,
  installedOnly,
}: {
  label: string;
  badge?: string;
  current: boolean;
  onClick: () => void;
  theme: string;
  colorTheme: string;
  fontTheme: string;
  installedOnly: boolean;
}) {
  const t = useT();
  return (
    <button className={`gallery-card theme-card${current ? " current" : ""}`} onClick={onClick} aria-pressed={current}>
      <Thumb theme={theme} colorTheme={colorTheme} fontTheme={fontTheme} installed={installedOnly} />
      <span className="theme-card-label">
        <strong>{label}</strong>
        {badge && <span className="muted theme-badge">{badge}</span>}
        {current && <Check size={14} className="theme-current" aria-label={t("dialogs.current")} />}
      </span>
    </button>
  );
}
