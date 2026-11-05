// The user's settings and recent files, kept in settings.json in the app's config
// folder. Unknown or missing values fall back to the defaults.
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";

export type Theme = "system" | "light" | "dark";
export type Density = "comfortable" | "compact";

export interface Settings {
  theme: Theme;
  density: Density;
  /** Page zoom in percent; 0 fits the page to the pane. */
  pageZoom: number;
  spellcheck: boolean;
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
}

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  density: "comfortable",
  pageZoom: 0,
  spellcheck: true,
  autoCompile: true,
  compileDelayMs: 700,
  crashRecovery: true,
  defaultFolder: "",
  recentLimit: 8,
  recentFiles: [],
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
  if (!["system", "light", "dark"].includes(out.theme)) out.theme = "system";
  if (!["comfortable", "compact"].includes(out.density)) out.density = "comfortable";
  out.compileDelayMs = Math.min(5000, Math.max(200, out.compileDelayMs));
  out.recentLimit = Math.min(20, Math.max(0, Math.round(out.recentLimit)));
  return out;
}

/** Theme and density are attributes on <html>, read by App.css. */
function applyToDocument(s: Settings) {
  const root = document.documentElement;
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
