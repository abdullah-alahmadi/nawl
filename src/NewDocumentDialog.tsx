// "New from template": a gallery of the built-in templates, grouped into reports
// and journals/conferences. Choosing one starts an empty document with it.
import { useEffect } from "react";
import { BookOpen, FileText, FolderOpen, GraduationCap, X } from "lucide-react";
import type { BuiltinTemplate } from "./api";

export function NewDocumentDialog({
  builtins,
  currentId,
  onPick,
  onOpenFile,
  onClose,
}: {
  builtins: BuiltinTemplate[];
  /** The active built-in template, if any. */
  currentId: string | null;
  onPick: (id: string) => void;
  onOpenFile: () => void;
  onClose: () => void;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const groups = [
    { title: "Reports", icon: <GraduationCap size={15} />, items: builtins.filter((b) => !b.class) },
    { title: "Journals and conferences", icon: <BookOpen size={15} />, items: builtins.filter((b) => b.class) },
  ];

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal gallery" role="dialog" aria-modal="true" aria-label="New from template">
        <div className="gallery-head">
          <h2>New from template</h2>
          <button className="icon-button" onClick={onClose} aria-label="Close" title="Close (Esc)">
            <X size={17} />
          </button>
        </div>
        <div className="gallery-body">
          {groups.map(
            (g) =>
              g.items.length > 0 && (
                <section key={g.title}>
                  <h3>
                    {g.icon}
                    {g.title}
                  </h3>
                  <div className="gallery-grid">
                    {g.items.map((b) => (
                      <button key={b.id} className={`gallery-card${b.id === currentId ? " current" : ""}`} onClick={() => onPick(b.id)}>
                        <span className="gallery-card-top">
                          <FileText size={18} aria-hidden="true" />
                          {b.class && <code>{b.class}</code>}
                        </span>
                        <strong>{b.name}</strong>
                        <span className="muted">{b.description}</span>
                      </button>
                    ))}
                  </div>
                </section>
              ),
          )}
        </div>
        <div className="gallery-foot">
          <button className="tb-button" onClick={onOpenFile}>
            <FolderOpen size={15} aria-hidden="true" /> Use a template file…
          </button>
          <span className="muted">The template can be changed later from the title bar.</span>
        </div>
      </div>
    </div>
  );
}
