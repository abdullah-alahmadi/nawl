// "Save as…" for templates: asks for a name and saves the template into My templates,
// where File → New from template… finds it. "Save to file…" is for people who keep a
// template.json elsewhere (to edit by hand, or under version control).
import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { useDraggable } from "../ui/useDraggable";
import { useT } from "../i18n";

export type SaveAsChoice = { kind: "library"; name: string } | { kind: "file" };

export function SaveTemplateDialog({
  defaultName,
  taken,
  title,
  onDone,
}: {
  defaultName: string;
  /** Names already in My templates (a second one is allowed, but said). */
  taken: string[];
  /** Why the template is being saved, when it isn't a plain Save as… */
  title?: string;
  /** The choice, or null when cancelled. */
  onDone: (choice: SaveAsChoice | null) => void;
}) {
  const t = useT();
  const [name, setName] = useState(defaultName);
  const input = useRef<HTMLInputElement>(null);
  const drag = useDraggable<HTMLDivElement>();
  const trimmed = name.trim();
  const duplicate = taken.some((n) => n.toLowerCase() === trimmed.toLowerCase());

  useEffect(() => {
    input.current?.select();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onDone(null);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onDone]);

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onDone(null)}>
      <div ref={drag.ref} style={drag.style} className="modal save-template-dialog" role="dialog" aria-modal="true" aria-label={t("designer.saveTemplateAs")}>
        <div className="gallery-head" {...drag.handle} title={t("designer.dragToMove")}>
          <h2>{t("designer.saveTemplateAs")}</h2>
          <button className="icon-button" onClick={() => onDone(null)} aria-label={t("common.close")} title={t("designer.closeEsc")}>
            <X size={17} />
          </button>
        </div>
        <form
          className="ref-form"
          onSubmit={(e) => {
            e.preventDefault();
            if (trimmed) onDone({ kind: "library", name: trimmed });
          }}
        >
          {title && <p className="save-template-why">{title}</p>}
          <label className="ref-row">
            <span>{t("common.name")}</span>
            <input ref={input} autoFocus value={name} onChange={(e) => setName(e.target.value)} spellCheck={false} />
          </label>
          <p className="muted ref-hint">
            {duplicate ? t("designer.saveDuplicate") : t("designer.saveLibraryHint")}
          </p>
          <div className="ref-foot">
            <button
              type="button"
              className="tb-button"
              onClick={() => onDone({ kind: "file" })}
              title={t("designer.saveToFileTitle")}
            >
              {t("designer.saveToFile")}
            </button>
            <span className="ref-problems" />
            <button type="button" className="tb-button" onClick={() => onDone(null)}>
              {t("common.cancel")}
            </button>
            <button type="submit" className="tb-button primary" disabled={!trimmed}>
              {t("common.save")}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
