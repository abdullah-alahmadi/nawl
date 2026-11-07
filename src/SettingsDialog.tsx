// Settings window (gear icon or Ctrl+,). Changes apply and save right away.
import { useEffect, useState, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FileText, Keyboard, Monitor, Moon, Palette, PenLine, Pipette, Sun, X } from "lucide-react";
import { DEFAULT_SETTINGS, PAGE_COLORS, type Settings } from "./settings";
import { listEngines, type EngineInfo } from "./api";
import { useT, type MessageKey } from "./i18n";

const ENGINE_LABELS: Record<string, MessageKey> = {
  tectonic: "dialogs.engineTectonic",
  pdflatex: "dialogs.enginePdflatex",
  xelatex: "dialogs.engineXelatex",
  lualatex: "dialogs.engineLualatex",
};

type Section = "editor" | "preview" | "appearance" | "files" | "shortcuts";

// Labels are message keys, translated when shown.
const SECTIONS: { key: Section; label: MessageKey; icon: ReactNode }[] = [
  { key: "editor", label: "dialogs.sectionEditor", icon: <PenLine size={16} /> },
  { key: "preview", label: "dialogs.sectionPreview", icon: <FileText size={16} /> },
  { key: "appearance", label: "dialogs.sectionAppearance", icon: <Palette size={16} /> },
  { key: "files", label: "dialogs.sectionFiles", icon: <FileText size={16} /> },
  { key: "shortcuts", label: "dialogs.sectionShortcuts", icon: <Keyboard size={16} /> },
];

/** The keys (a message key when they include words) and what they do. */
const SHORTCUTS: [string | { key: MessageKey }, MessageKey][] = [
  ["Ctrl+N", "dialogs.shortcutNewEmpty"],
  ["Ctrl+Shift+N", "dialogs.newFromTemplate"],
  ["Ctrl+O", "common.open"],
  ["Ctrl+S", "common.save"],
  ["Ctrl+Shift+S", "dialogs.shortcutSaveAs"],
  ["Ctrl+Shift+E", "dialogs.shortcutExportPdf"],
  ["Ctrl+Enter", "dialogs.shortcutCompileNow"],
  ["Ctrl+,", "dialogs.settings"],
  ["Ctrl+Z / Ctrl+Shift+Z", "dialogs.shortcutUndoRedo"],
  ["Ctrl+B / Ctrl+I / Ctrl+E", "dialogs.shortcutBoldItalicCode"],
  ["Ctrl+M", "dialogs.shortcutInlineMath"],
  [{ key: "dialogs.shortcutDisplayMathKeys" }, "dialogs.shortcutDisplayMath"],
  ["Esc", "dialogs.shortcutLeaveFormula"],
];

export function SettingsDialog({
  settings,
  onChange,
  onClose,
}: {
  settings: Settings;
  onChange: (patch: Partial<Settings>) => void;
  onClose: () => void;
}) {
  const t = useT();
  const [section, setSection] = useState<Section>("editor");
  const [engines, setEngines] = useState<EngineInfo[] | null>(null);
  useEffect(() => {
    listEngines().then(setEngines, () => setEngines([]));
  }, []);
  const current = engines?.find((e) => e.id === settings.engine);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const engineLabel = (id: string) => (ENGINE_LABELS[id] ? t(ENGINE_LABELS[id]) : id);

  const chooseFolder = async () => {
    const dir = await open({ directory: true, multiple: false, defaultPath: settings.defaultFolder || undefined });
    if (typeof dir === "string") onChange({ defaultFolder: dir });
  };

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal settings" role="dialog" aria-modal="true" aria-label={t("dialogs.settings")}>
        <nav className="settings-nav" aria-label={t("dialogs.settingsSections")}>
          <div className="settings-title">{t("dialogs.settings")}</div>
          {SECTIONS.map((s) => (
            <button key={s.key} className={section === s.key ? "active" : ""} onClick={() => setSection(s.key)}>
              {s.icon}
              {t(s.label)}
            </button>
          ))}
        </nav>
        <div className="settings-body">
          <div className="settings-head">
            <h2>{t(SECTIONS.find((s) => s.key === section)!.label)}</h2>
            <button className="icon-button" onClick={onClose} aria-label={t("dialogs.closeSettings")} title={t("dialogs.closeEsc")}>
              <X size={17} />
            </button>
          </div>

          {section === "editor" && (
            <>
              <Field label={t("dialogs.pageZoom")} hint={t("dialogs.pageZoomHint")}>
                <select value={settings.pageZoom} onChange={(e) => onChange({ pageZoom: Number(e.target.value) })}>
                  <option value={0}>{t("dialogs.fitToPane")}</option>
                  {[75, 90, 100, 110, 125, 150].map((z) => (
                    <option key={z} value={z}>
                      {z}%
                    </option>
                  ))}
                </select>
              </Field>
              <Field label={t("dialogs.pageColor")} hint={t("dialogs.pageColorHint")}>
                <span className="swatches">
                  {PAGE_COLORS.map((c) => (
                    <button
                      key={c.color}
                      className={`swatch${settings.pageColor.toLowerCase() === c.color ? " active" : ""}`}
                      style={{ background: c.color }}
                      title={t(c.name)}
                      aria-label={t(c.name)}
                      aria-pressed={settings.pageColor.toLowerCase() === c.color}
                      onClick={() => onChange({ pageColor: c.color })}
                    />
                  ))}
                  <label
                    className={`swatch-custom${PAGE_COLORS.some((c) => c.color === settings.pageColor.toLowerCase()) ? "" : " active"}`}
                    title={t("dialogs.customColor")}
                    style={PAGE_COLORS.some((c) => c.color === settings.pageColor.toLowerCase()) ? undefined : { background: settings.pageColor }}
                  >
                    <Pipette size={13} aria-hidden="true" />
                    <input type="color" value={settings.pageColor} onChange={(e) => onChange({ pageColor: e.target.value })} aria-label={t("dialogs.customPageColor")} />
                  </label>
                </span>
              </Field>
              <Field label={t("dialogs.spellChecking")} hint={t("dialogs.spellCheckingHint")}>
                <Switch checked={settings.spellcheck} onChange={(v) => onChange({ spellcheck: v })} />
              </Field>
              <Field label={t("dialogs.arabicPunctuation")} hint={t("dialogs.arabicPunctuationHint")}>
                <Switch checked={settings.arabicPunctuation} onChange={(v) => onChange({ arabicPunctuation: v })} />
              </Field>
              <Field label={t("dialogs.crashRecovery")} hint={t("dialogs.crashRecoveryHint")}>
                <Switch checked={settings.crashRecovery} onChange={(v) => onChange({ crashRecovery: v })} />
              </Field>
            </>
          )}

          {section === "preview" && (
            <>
              <Field label={t("dialogs.texEngine")} hint={t("dialogs.texEngineHint")}>
                <select value={settings.engine} onChange={(e) => onChange({ engine: e.target.value })} aria-label={t("dialogs.texEngine")}>
                  {(engines ?? [{ id: settings.engine, available: true, detail: null, latexmk: false }]).map((e) => (
                    <option key={e.id} value={e.id} disabled={!e.available && e.id !== settings.engine}>
                      {engineLabel(e.id)}
                      {e.available ? "" : t("dialogs.notInstalledSuffix")}
                    </option>
                  ))}
                </select>
              </Field>
              <div className="settings-detail">
                {engines === null
                  ? t("dialogs.lookingForTex")
                  : current?.available
                    ? `${current.detail ?? ""}${current.id !== "tectonic" ? (current.latexmk ? t("dialogs.runsThroughLatexmk") : t("dialogs.latexmkNotFound")) : ""}`
                    : t("dialogs.engineNotInstalled", { engine: engineLabel(settings.engine) })}
                {engines && !engines.some((e) => e.id !== "tectonic" && e.available) && t("dialogs.noInstalledTex")}
              </div>
              <Field label={t("dialogs.compileWhileTyping")} hint={t("dialogs.compileWhileTypingHint")}>
                <Switch checked={settings.autoCompile} onChange={(v) => onChange({ autoCompile: v })} />
              </Field>
              <Field label={t("dialogs.compileDelay")} hint={t("dialogs.compileDelayHint")}>
                <span className="settings-range">
                  <input
                    type="range"
                    min={200}
                    max={3000}
                    step={100}
                    value={settings.compileDelayMs}
                    disabled={!settings.autoCompile}
                    onChange={(e) => onChange({ compileDelayMs: Number(e.target.value) })}
                  />
                  <span>{t("dialogs.seconds", { n: (settings.compileDelayMs / 1000).toFixed(1) })}</span>
                </span>
              </Field>
            </>
          )}

          {section === "appearance" && (
            <>
              <Field label={t("dialogs.language")} hint={t("dialogs.languageHint")}>
                <div className="segmented">
                  {(
                    [
                      // Each language names itself, in either interface language.
                      ["system", t("dialogs.system")],
                      ["en", "English"],
                      ["ar", "العربية"],
                    ] as const
                  ).map(([v, label]) => (
                    <button key={v} className={settings.uiLanguage === v ? "active" : ""} onClick={() => onChange({ uiLanguage: v })}>
                      {label}
                    </button>
                  ))}
                </div>
              </Field>
              <Field label={t("dialogs.theme")} hint={t("dialogs.themeHint")}>
                <div className="segmented">
                  {(
                    [
                      ["system", t("dialogs.system"), <Monitor size={15} key="m" />],
                      ["light", t("dialogs.themeLight"), <Sun size={15} key="s" />],
                      ["dark", t("dialogs.themeDark"), <Moon size={15} key="d" />],
                    ] as const
                  ).map(([v, label, icon]) => (
                    <button key={v} className={settings.theme === v ? "active" : ""} onClick={() => onChange({ theme: v })}>
                      {icon}
                      {label}
                    </button>
                  ))}
                </div>
              </Field>
              <Field label={t("dialogs.density")} hint={t("dialogs.densityHint")}>
                <div className="segmented">
                  {(["comfortable", "compact"] as const).map((v) => (
                    <button key={v} className={settings.density === v ? "active" : ""} onClick={() => onChange({ density: v })}>
                      {v === "comfortable" ? t("dialogs.densityComfortable") : t("dialogs.densityCompact")}
                    </button>
                  ))}
                </div>
              </Field>
            </>
          )}

          {section === "files" && (
            <>
              <Field label={t("dialogs.defaultFolder")} hint={t("dialogs.defaultFolderHint")}>
                <span className="settings-folder">
                  <code title={settings.defaultFolder}>{settings.defaultFolder || t("dialogs.systemDefault")}</code>
                  <button className="tb-button" onClick={chooseFolder}>
                    {t("dialogs.choose")}
                  </button>
                  {settings.defaultFolder && (
                    <button className="tb-button" onClick={() => onChange({ defaultFolder: "" })}>
                      {t("common.reset")}
                    </button>
                  )}
                </span>
              </Field>
              <Field label={t("dialogs.recentFiles")} hint={t("dialogs.recentFilesHint")}>
                <span className="settings-folder">
                  <input
                    type="number"
                    min={0}
                    max={20}
                    value={settings.recentLimit}
                    onChange={(e) => onChange({ recentLimit: Math.min(20, Math.max(0, Number(e.target.value) || 0)) })}
                  />
                  <button className="tb-button" onClick={() => onChange({ recentFiles: [] })} disabled={settings.recentFiles.length === 0}>
                    {t("dialogs.clearList")}
                  </button>
                </span>
              </Field>
            </>
          )}

          {section === "shortcuts" && (
            <table className="shortcuts">
              <tbody>
                {SHORTCUTS.map(([keys, what]) => (
                  <tr key={what}>
                    <td>
                      <kbd>{typeof keys === "string" ? keys : t(keys.key)}</kbd>
                    </td>
                    <td>{t(what)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          {section !== "shortcuts" && (
            <div className="settings-foot">
              <button
                className="link"
                onClick={() => {
                  // Lists of what was used recently stay.
                  const { recentFiles, recentTemplates, ...defaults } = DEFAULT_SETTINGS;
                  void recentFiles;
                  void recentTemplates;
                  onChange(defaults);
                }}
              >
                {t("dialogs.restoreDefaults")}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function Field({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="settings-field">
      <div>
        <div className="settings-label">{label}</div>
        {hint && <div className="settings-hint">{hint}</div>}
      </div>
      <div className="settings-control">{children}</div>
    </div>
  );
}

function Switch({ checked, onChange }: { checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <button role="switch" aria-checked={checked} className={`switch${checked ? " on" : ""}`} onClick={() => onChange(!checked)}>
      <span />
    </button>
  );
}
