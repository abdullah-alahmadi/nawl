// The user's settings and recent files, kept in settings.json in the app's config
// folder. Unknown or missing values fall back to the defaults.
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import { resolveUiLanguage, setUiLanguage, type MessageKey } from "./i18n";

export type Theme = "system" | "light" | "dark";
export type Density = "comfortable" | "compact";

export interface Settings {
  /** Interface language: "system" (the OS language), "en" or "ar". */
  uiLanguage: string;
  theme: Theme;
  density: Density;
  /** Page zoom in percent; 0 fits the page to the pane. */
  pageZoom: number;
  spellcheck: boolean;
  /** In Arabic text, type ؟ ، ؛ and «» for ? , ; and ". */
  arabicPunctuation: boolean;
  /** Background of the editor page (#rrggbb). The PDF preview keeps real white paper. */
  pageColor: string;
  /** TeX engine: the bundled "tectonic", or an installed "pdflatex", "xelatex" or "lualatex". */
  engine: string;
  /** Recompile the preview while typing. */
  autoCompile: boolean;
  /** Pause after typing before recompiling, in milliseconds. */
  compileDelayMs: number;
  /** Keep a copy of unsaved changes for crash recovery. */
  crashRecovery: boolean;
  /** Folder the Open and Save dialogs start in; empty for the system default. */
  defaultFolder: string;
  recentLimit: number;
  /** Most recent first. */
  recentFiles: string[];
  /** The template window: cards or a compact list. */
  galleryView: "grid" | "list";
  /** The template window's category ("all", "recent", "journal"…). */
  galleryCategory: string;
  /** Templates documents were last started with ("builtin:id", "tex:class", "mine:path"). */
  recentTemplates: string[];
}

export const DEFAULT_SETTINGS: Settings = {
  uiLanguage: "system",
  theme: "system",
  density: "comfortable",
  pageZoom: 0,
  spellcheck: true,
  arabicPunctuation: true,
  pageColor: "#f4efe6",
  engine: "tectonic",
  autoCompile: true,
  compileDelayMs: 700,
  crashRecovery: true,
  defaultFolder: "",
  recentLimit: 8,
  recentFiles: [],
  galleryView: "grid",
  galleryCategory: "all",
  recentTemplates: [],
};

/** Keeps only values of the right type, so a hand-edited file can't break the app. */
function sanitize(raw: unknown): Settings {
  const out = { ...DEFAULT_SETTINGS };
  if (!raw || typeof raw !== "object") return out;
  const r = raw as Record<string, unknown>;
  for (const key of Object.keys(DEFAULT_SETTINGS) as (keyof Settings)[]) {
    const def = DEFAULT_SETTINGS[key];
    const v = r[key];
    if (Array.isArray(def)) {
      if (Array.isArray(v)) (out as Record<string, unknown>)[key] = v.filter((x) => typeof x === "string");
    } else if (typeof v === typeof def) {
      (out as Record<string, unknown>)[key] = v;
    }
  }
  if (!["system", "en", "ar"].includes(out.uiLanguage)) out.uiLanguage = "system";
  if (!["system", "light", "dark"].includes(out.theme)) out.theme = "system";
  if (!["comfortable", "compact"].includes(out.density)) out.density = "comfortable";
  out.compileDelayMs = Math.min(5000, Math.max(200, out.compileDelayMs));
  if (!["tectonic", "pdflatex", "xelatex", "lualatex"].includes(out.engine)) out.engine = "tectonic";
  if (!/^#[0-9a-f]{6}$/i.test(out.pageColor)) out.pageColor = DEFAULT_SETTINGS.pageColor;
  out.recentLimit = Math.min(20, Math.max(0, Math.round(out.recentLimit)));
  if (!["grid", "list"].includes(out.galleryView)) out.galleryView = "grid";
  out.recentTemplates = out.recentTemplates.slice(0, 5);
  return out;
}

/** Page colour presets for the settings window (names are translated when shown). */
export const PAGE_COLORS: { name: MessageKey; color: string }[] = [
  { name: "dialogs.pageColorWarmPaper", color: "#f4efe6" },
  { name: "dialogs.pageColorSepia", color: "#efe3c8" },
  { name: "dialogs.pageColorSoftGray", color: "#e9ebee" },
  { name: "dialogs.pageColorMint", color: "#e5efe7" },
  { name: "dialogs.pageColorSky", color: "#e6eef6" },
  { name: "dialogs.pageColorWhite", color: "#ffffff" },
  { name: "dialogs.pageColorSlate", color: "#2b2f36" },
  { name: "dialogs.pageColorNight", color: "#1c1d21" },
];

/** Relative luminance (0 black – 1 white) of a #rrggbb colour. */
export function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Language, theme, density and page colour are set on <html>, read by App.css. */
function applyToDocument(s: Settings) {
  const root = document.documentElement;
  // The interface direction; the editor page keeps the document's own.
  const lang = resolveUiLanguage(s.uiLanguage);
  setUiLanguage(lang);
  api.setMessageLanguage(lang === "ar").catch(() => {});
  root.lang = lang;
  root.dir = lang === "ar" ? "rtl" : "ltr";
  // Dark pages get light text (and a few adjusted colours, see .page-dark in App.css).
  const dark = luminance(s.pageColor) < 0.18;
  root.style.setProperty("--page-bg", s.pageColor);
  root.style.setProperty("--page-fg", dark ? "#e4e6ea" : "#1d2228");
  if (dark) root.dataset.pageDark = "";
  else delete root.dataset.pageDark;
  if (s.theme === "system") root.removeAttribute("data-theme");
  else root.dataset.theme = s.theme;
  root.dataset.density = s.density;
}

export function useSettings() {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [loaded, setLoaded] = useState(false);
  const latest = useRef(settings);

  useEffect(() => {
    api.getSettings().then(
      (raw) => {
        const s = sanitize(raw);
        latest.current = s;
        setSettings(s);
        applyToDocument(s);
        setLoaded(true);
      },
      () => setLoaded(true),
    );
  }, []);

  const update = useCallback((patch: Partial<Settings> | ((s: Settings) => Partial<Settings>)) => {
    const next = { ...latest.current, ...(typeof patch === "function" ? patch(latest.current) : patch) };
    next.recentFiles = next.recentFiles.slice(0, next.recentLimit);
    latest.current = next;
    setSettings(next);
    applyToDocument(next);
    api.saveSettings(next).catch((e) => console.warn("settings not saved:", e));
  }, []);

  /** Puts a file at the top of the recent list. */
  const addRecent = useCallback(
    (path: string) => update((s) => ({ recentFiles: [path, ...s.recentFiles.filter((p) => p !== path)] })),
    [update],
  );
  const removeRecent = useCallback(
    (path: string) => update((s) => ({ recentFiles: s.recentFiles.filter((p) => p !== path) })),
    [update],
  );

  return { settings, loaded, update, addRecent, removeRecent, latest };
}
