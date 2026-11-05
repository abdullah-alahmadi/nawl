// Settings window (gear icon or Ctrl+,). Changes apply and save right away.
import { useEffect, useState, type ReactNode } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FileText, Keyboard, Monitor, Moon, Palette, PenLine, Sun, X } from "lucide-react";
import { DEFAULT_SETTINGS, type Settings } from "./settings";

type Section = "editor" | "preview" | "appearance" | "files" | "shortcuts";

const SECTIONS: { key: Section; label: string; icon: ReactNode }[] = [
  { key: "editor", label: "Editor", icon: <PenLine size={16} /> },
  { key: "preview", label: "Preview", icon: <FileText size={16} /> },
  { key: "appearance", label: "Appearance", icon: <Palette size={16} /> },
  { key: "files", label: "Files", icon: <FileText size={16} /> },
  { key: "shortcuts", label: "Keyboard shortcuts", icon: <Keyboard size={16} /> },
];

const SHORTCUTS: [string, string][] = [
  ["Ctrl+N", "New empty document"],
  ["Ctrl+Shift+N", "New from template"],
  ["Ctrl+O", "Open"],
  ["Ctrl+S", "Save"],
  ["Ctrl+Shift+S", "Save as"],
  ["Ctrl+Shift+E", "Export PDF"],
  ["Ctrl+Enter", "Compile now"],
  ["Ctrl+,", "Settings"],
  ["Ctrl+Z / Ctrl+Shift+Z", "Undo / redo"],
  ["Ctrl+B / Ctrl+I / Ctrl+E", "Bold / italic / code"],
  ["Ctrl+M", "Inline math (or type $x^2$)"],
  ["$$ then Space", "Equation on its own line"],
  ["Esc", "Leave a formula or LaTeX block"],
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
  const [section, setSection] = useState<Section>("editor");

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const chooseFolder = async () => {
    const dir = await open({ directory: true, multiple: false, defaultPath: settings.defaultFolder || undefined });
    if (typeof dir === "string") onChange({ defaultFolder: dir });
  };

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal settings" role="dialog" aria-modal="true" aria-label="Settings">
        <nav className="settings-nav" aria-label="Settings sections">
          <div className="settings-title">Settings</div>
          {SECTIONS.map((s) => (
            <button key={s.key} className={section === s.key ? "active" : ""} onClick={() => setSection(s.key)}>
              {s.icon}
              {s.label}
            </button>
          ))}
        </nav>
        <div className="settings-body">
          <div className="settings-head">
            <h2>{SECTIONS.find((s) => s.key === section)?.label}</h2>
            <button className="icon-button" onClick={onClose} aria-label="Close settings" title="Close (Esc)">
              <X size={17} />
            </button>
          </div>

          {section === "editor" && (
            <>
              <Field label="Page zoom" hint="How large the page is drawn while you write.">
                <select value={settings.pageZoom} onChange={(e) => onChange({ pageZoom: Number(e.target.value) })}>
                  <option value={0}>Fit to the pane</option>
                  {[75, 90, 100, 110, 125, 150].map((z) => (
                    <option key={z} value={z}>
                      {z}%
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Spell checking" hint="Underline misspelled words while typing.">
                <Switch checked={settings.spellcheck} onChange={(v) => onChange({ spellcheck: v })} />
              </Field>
              <Field label="Crash recovery" hint="Keep a copy of unsaved changes, offered back if Texloom closes unexpectedly.">
                <Switch checked={settings.crashRecovery} onChange={(v) => onChange({ crashRecovery: v })} />
              </Field>
            </>
          )}

          {section === "preview" && (
            <>
              <Field label="Compile while typing" hint="Off: the preview updates when you press Compile (Ctrl+Enter).">
                <Switch checked={settings.autoCompile} onChange={(v) => onChange({ autoCompile: v })} />
              </Field>
              <Field label="Delay before compiling" hint="A longer pause saves work on slow computers.">
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
                  <span>{(settings.compileDelayMs / 1000).toFixed(1)} s</span>
                </span>
              </Field>
            </>
          )}

          {section === "appearance" && (
            <>
              <Field label="Theme" hint="The page always stays white, like paper.">
                <div className="segmented">
                  {(
                    [
                      ["system", "System", <Monitor size={15} key="m" />],
                      ["light", "Light", <Sun size={15} key="s" />],
                      ["dark", "Dark", <Moon size={15} key="d" />],
                    ] as const
                  ).map(([v, label, icon]) => (
                    <button key={v} className={settings.theme === v ? "active" : ""} onClick={() => onChange({ theme: v })}>
                      {icon}
                      {label}
                    </button>
                  ))}
                </div>
              </Field>
              <Field label="Density" hint="Compact fits more on small screens.">
                <div className="segmented">
                  {(["comfortable", "compact"] as const).map((v) => (
                    <button key={v} className={settings.density === v ? "active" : ""} onClick={() => onChange({ density: v })}>
                      {v === "comfortable" ? "Comfortable" : "Compact"}
                    </button>
                  ))}
                </div>
              </Field>
            </>
          )}

          {section === "files" && (
            <>
              <Field label="Default folder" hint="Where Open and Save start.">
                <span className="settings-folder">
                  <code title={settings.defaultFolder}>{settings.defaultFolder || "System default"}</code>
                  <button className="tb-button" onClick={chooseFolder}>
                    Choose…
                  </button>
                  {settings.defaultFolder && (
                    <button className="tb-button" onClick={() => onChange({ defaultFolder: "" })}>
                      Reset
                    </button>
                  )}
                </span>
              </Field>
              <Field label="Recent files" hint="How many to list in the File menu.">
                <span className="settings-folder">
                  <input
                    type="number"
                    min={0}
                    max={20}
                    value={settings.recentLimit}
                    onChange={(e) => onChange({ recentLimit: Math.min(20, Math.max(0, Number(e.target.value) || 0)) })}
                  />
                  <button className="tb-button" onClick={() => onChange({ recentFiles: [] })} disabled={settings.recentFiles.length === 0}>
                    Clear list
                  </button>
                </span>
              </Field>
            </>
          )}

          {section === "shortcuts" && (
            <table className="shortcuts">
              <tbody>
                {SHORTCUTS.map(([keys, what]) => (
                  <tr key={keys}>
                    <td>
                      <kbd>{keys}</kbd>
                    </td>
                    <td>{what}</td>
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
                  const { recentFiles, ...defaults } = DEFAULT_SETTINGS;
                  void recentFiles;
                  onChange(defaults);
                }}
              >
                Restore defaults
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
