// "New from template": a gallery of the built-in templates, grouped into reports
// and journals/conferences. Choosing one starts an empty document with it.
import { useEffect } from "react";
import { BookOpen, FileInput, FileText, FolderHeart, GraduationCap, Trash2, X } from "lucide-react";
import type { BuiltinTemplate, UserTemplate } from "./api";

export function NewDocumentDialog({
  builtins,
  currentId,
  onPick,
  userTemplates,
  currentPath,
  onPickUser,
  onRemoveUser,
  onImport,
  onClose,
}: {
  builtins: BuiltinTemplate[];
  /** The active built-in template, if any. */
  currentId: string | null;
  onPick: (id: string) => void;
  userTemplates: UserTemplate[];
  /** File of the active template, if it isn't built in. */
  currentPath: string | null;
  onPickUser: (t: UserTemplate) => void;
  onRemoveUser: (t: UserTemplate) => void;
  onImport: () => void;
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
          <section>
            <h3>
              <FolderHeart size={15} />
              My templates
            </h3>
            {userTemplates.length === 0 ? (
              <button className="gallery-import" onClick={onImport}>
                <FileInput size={20} aria-hidden="true" />
                <span>
                  <strong>Import a template for any journal</strong>
                  <span className="muted">
                    Choose the publisher’s LaTeX kit (.zip from the journal’s author guidelines), a template package
                    someone shared, or a Texloom template (.json).
                  </span>
                </span>
              </button>
            ) : (
              <div className="gallery-grid">
                {userTemplates.map((u) => (
                  <div key={u.id} className={`gallery-card user${u.path === currentPath ? " current" : ""}`}>
                    <button className="gallery-card-main" onClick={() => onPickUser(u)}>
                      <span className="gallery-card-top">
                        <FileText size={18} aria-hidden="true" />
                        {u.class && <code>{u.class}</code>}
                      </span>
                      <strong>{u.name}</strong>
                      <span className="muted">{u.description}</span>
                    </button>
                    <button className="icon-button small gallery-remove" onClick={() => onRemoveUser(u)} title="Remove from My templates" aria-label={`Remove ${u.name}`}>
                      <Trash2 size={14} />
                    </button>
                  </div>
                ))}
              </div>
            )}
          </section>
        </div>
        <div className="gallery-foot">
          <button className="tb-button" onClick={onImport}>
            <FileInput size={15} aria-hidden="true" /> Import template…
          </button>
          <span className="muted">A publisher’s LaTeX kit, a template package, or a template file. The template can be changed later from the title bar.</span>
        </div>
      </div>
    </div>
  );
}
