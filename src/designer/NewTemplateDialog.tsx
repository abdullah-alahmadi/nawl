// "New blank template": a template to design from scratch. It asks for a name and
// the kind of document, then saves the plain template into My templates (so images
// can be added at once) and opens it in the designer.
import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { useDraggable } from "../ui/useDraggable";
import type { BlankSpec } from "../api";
import { useT, type MessageKey } from "../i18n";

type Kind = "report" | "article" | "presentation";

// Labels are message keys, translated when drawn.
const KINDS: { value: Kind; label: MessageKey; hint: MessageKey }[] = [
  { value: "report", label: "designer.kindReport", hint: "designer.kindReportHint" },
  { value: "article", label: "designer.kindArticle", hint: "designer.kindArticleHint" },
  { value: "presentation", label: "designer.kindPresentation", hint: "designer.kindPresentationHint" },
];

export function NewTemplateDialog({ onCreate, onClose }: { onCreate: (name: string, spec: BlankSpec) => Promise<boolean>; onClose: () => void }) {
  const t = useT();
  const [name, setName] = useState(() => t("designer.defaultTemplateName"));
  const [kind, setKind] = useState<Kind>("report");
  const [paper, setPaper] = useState<"a4" | "letter">("a4");
  const [arabic, setArabic] = useState(() => navigator.language.toLowerCase().startsWith("ar"));
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const drag = useDraggable<HTMLDivElement>();

  useEffect(() => {
    input.current?.select();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const create = async () => {
    if (!name.trim() || busy) return;
    setBusy(true);
    const ok = await onCreate(name.trim(), { presentation: kind === "presentation", paper: { kind: paper }, chapters: kind === "report", arabic });
    setBusy(false);
    if (ok) onClose();
  };

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div ref={drag.ref} style={drag.style} className="modal save-template-dialog" role="dialog" aria-modal="true" aria-label={t("designer.newTemplateTitle")}>
        <div className="gallery-head" {...drag.handle} title={t("designer.dragToMove")}>
          <h2>{t("designer.newTemplateTitle")}</h2>
          <button className="icon-button" onClick={onClose} aria-label={t("common.close")} title={t("designer.closeEsc")}>
            <X size={17} />
          </button>
        </div>
        <form
          className="ref-form"
          onSubmit={(e) => {
            e.preventDefault();
            create();
          }}
        >
          <p className="save-template-why">{t("designer.newTemplateIntro")}</p>
          <label className="ref-row">
            <span>{t("common.name")}</span>
            <input ref={input} autoFocus value={name} onChange={(e) => setName(e.target.value)} spellCheck={false} />
          </label>
          <div className="ref-row" role="radiogroup" aria-label={t("designer.kindAria")}>
            <span>{t("designer.kind")}</span>
            <div className="new-template-kinds">
              {KINDS.map((k) => (
                <label key={k.value} className={`new-template-kind${kind === k.value ? " active" : ""}`}>
                  <input type="radio" name="kind" checked={kind === k.value} onChange={() => setKind(k.value)} />
                  <strong>{t(k.label)}</strong>
                  <span className="muted">{t(k.hint)}</span>
                </label>
              ))}
            </div>
          </div>
          <label className="ref-row">
            <span>{t("designer.language")}</span>
            <select value={arabic ? "arabic" : "english"} onChange={(e) => setArabic(e.target.value === "arabic")}>
              <option value="english">{t("designer.langEnglish")}</option>
              <option value="arabic">{t("designer.langArabic")}</option>
            </select>
          </label>
          {kind !== "presentation" && (
            <label className="ref-row">
              <span>{t("designer.paper")}</span>
              <select value={paper} onChange={(e) => setPaper(e.target.value as "a4" | "letter")}>
                <option value="a4">A4</option>
                <option value="letter">US Letter</option>
              </select>
            </label>
          )}
          <p className="muted ref-hint">{t("designer.newTemplateSavedHint")}</p>
          <div className="ref-foot">
            <span className="ref-problems" />
            <button type="button" className="tb-button" onClick={onClose}>
              {t("common.cancel")}
            </button>
            <button type="submit" className="tb-button primary" disabled={!name.trim() || busy}>
              {t("common.create")}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
