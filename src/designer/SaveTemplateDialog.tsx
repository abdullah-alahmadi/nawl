// "Save as…" for templates: asks for a name and saves the template into My templates,
// where File → New from template… finds it. "Save to file…" is for people who keep a
// template.json elsewhere (to edit by hand, or under version control).
import { useEffect, useRef, useState } from "react";
import { X } from "lucide-react";
import { useDraggable } from "../ui/useDraggable";

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
  const [name, setName] = useState(defaultName);
  const input = useRef<HTMLInputElement>(null);
  const drag = useDraggable<HTMLDivElement>();
  const trimmed = name.trim();
  const duplicate = taken.some((t) => t.toLowerCase() === trimmed.toLowerCase());

  useEffect(() => {
    input.current?.select();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onDone(null);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onDone]);

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onDone(null)}>
      <div ref={drag.ref} style={drag.style} className="modal save-template-dialog" role="dialog" aria-modal="true" aria-label="Save template as">
        <div className="gallery-head" {...drag.handle} title="Drag to move">
          <h2>Save template as</h2>
          <button className="icon-button" onClick={() => onDone(null)} aria-label="Close" title="Close (Esc)">
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
            <span>Name</span>
            <input ref={input} autoFocus value={name} onChange={(e) => setName(e.target.value)} spellCheck={false} />
          </label>
          <p className="muted ref-hint">
            {duplicate
              ? "My templates already has a template with this name; this saves another one beside it."
              : "Saved in My templates, so it's ready under File → New from template…"}
          </p>
          <div className="ref-foot">
            <button
              type="button"
              className="tb-button"
              onClick={() => onDone({ kind: "file" })}
              title="Save a template.json wherever you like (to edit by hand); open it again with Open template file…"
            >
              Save to file…
            </button>
            <span className="ref-problems" />
            <button type="button" className="tb-button" onClick={() => onDone(null)}>
              Cancel
            </button>
            <button type="submit" className="tb-button primary" disabled={!trimmed}>
              Save
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
