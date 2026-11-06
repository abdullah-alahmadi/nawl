// "New blank template": a template to design from scratch. It asks for a name and
// the kind of document, then saves the plain template into My templates (so images
// can be added at once) and opens it in the designer.
import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { useDraggable } from "../ui/useDraggable";
import type { BlankSpec } from "../api";

type Kind = "report" | "article" | "presentation";

const KINDS: { value: Kind; label: string; hint: string }[] = [
  { value: "report", label: "Report or thesis", hint: "Chapters, a title page and a table of contents." },
  { value: "article", label: "Article", hint: "Sections, a title page." },
  { value: "presentation", label: "Presentation", hint: "Beamer slides; theme and colours under Slides." },
];

export function NewTemplateDialog({ onCreate, onClose }: { onCreate: (name: string, spec: BlankSpec) => Promise<boolean>; onClose: () => void }) {
  const [name, setName] = useState("My template");
  const [kind, setKind] = useState<Kind>("report");
  const [paper, setPaper] = useState<"a4" | "letter">("a4");
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
    const ok = await onCreate(name.trim(), { presentation: kind === "presentation", paper: { kind: paper }, chapters: kind === "report" });
    setBusy(false);
    if (ok) onClose();
  };

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div ref={drag.ref} style={drag.style} className="modal save-template-dialog" role="dialog" aria-modal="true" aria-label="New blank template">
        <div className="gallery-head" {...drag.handle} title="Drag to move">
          <h2>New blank template</h2>
          <button className="icon-button" onClick={onClose} aria-label="Close" title="Close (Esc)">
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
          <p className="save-template-why">A plain template to design from scratch: a title, authors and date, simple headings, and nothing else.</p>
          <label className="ref-row">
            <span>Name</span>
            <input ref={input} autoFocus value={name} onChange={(e) => setName(e.target.value)} spellCheck={false} />
          </label>
          <div className="ref-row" role="radiogroup" aria-label="Kind of document">
            <span>Kind</span>
            <div className="new-template-kinds">
              {KINDS.map((k) => (
                <label key={k.value} className={`new-template-kind${kind === k.value ? " active" : ""}`}>
                  <input type="radio" name="kind" checked={kind === k.value} onChange={() => setKind(k.value)} />
                  <strong>{k.label}</strong>
                  <span className="muted">{k.hint}</span>
                </label>
              ))}
            </div>
          </div>
          {kind !== "presentation" && (
            <label className="ref-row">
              <span>Paper</span>
              <select value={paper} onChange={(e) => setPaper(e.target.value as "a4" | "letter")}>
                <option value="a4">A4</option>
                <option value="letter">US Letter</option>
              </select>
            </label>
          )}
          <p className="muted ref-hint">Saved in My templates and opened in the designer. Everything can be changed there.</p>
          <div className="ref-foot">
            <span className="ref-problems" />
            <button type="button" className="tb-button" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" className="tb-button primary" disabled={!name.trim() || busy}>
              Create
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
