// "New from template": a gallery of the built-in templates, grouped into reports
// and journals/conferences, the user's own templates, and the journal classes of
// the installed TeX distribution. Choosing one starts an empty document with it. At
// startup it is the start window, with Open, recent files and the example as well.
import { useEffect, useMemo, useState } from "react";
import { BookOpen, FileInput, FileText, FolderHeart, FolderOpen, GraduationCap, History, Library, Presentation, Search, Sparkles, Trash2, X } from "lucide-react";
import type { BuiltinTemplate, InstalledCategory, InstalledList, InstalledTemplate, UserTemplate } from "./api";

/** The groups of the "From your TeX installation" section, in order. */
const TEX_GROUPS: { category: InstalledCategory; title: string }[] = [
  { category: "journal", title: "Journals and conferences" },
  { category: "thesis", title: "Theses and dissertations" },
  { category: "book", title: "Books" },
  { category: "general", title: "Articles and reports" },
];

/** Names for the kinds Texloom can't write yet. */
const WAITING_NAMES: Partial<Record<InstalledCategory, string>> = {
  letter: "letter",
  cv: "CV",
  poster: "poster",
  exam: "exam and exercise",
  presentation: "presentation (other than beamer)",
};

/** Whether a template matches the search box (name, description or class). */
function matches(query: string, ...texts: (string | null | undefined)[]) {
  const q = query.trim().toLowerCase();
  return !q || texts.some((t) => t?.toLowerCase().includes(q));
}

export function NewDocumentDialog({
  builtins,
  currentId,
  onPick,
  userTemplates,
  currentPath,
  onPickUser,
  onRemoveUser,
  onImport,
  installed = [],
  waiting = [],
  installedThemes = 0,
  currentClass = null,
  onPickInstalled,
  engine = "tectonic",
  only,
  start = false,
  recentFiles = [],
  onOpen,
  onExample,
  onExamplePresentation,
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
  /** Templates from the installed TeX distribution. */
  installed?: InstalledTemplate[];
  /** Installed classes of kinds Texloom can't write yet. */
  waiting?: InstalledList["waiting"];
  /** Beamer themes the installed TeX adds to the theme picker. */
  installedThemes?: number;
  /** Class of the active installed template, if any. */
  currentClass?: string | null;
  onPickInstalled?: (cls: string) => void;
  /** The TeX engine from the settings. */
  engine?: string;
  /** "installed": only the installed-TeX templates, to restyle the open document
   *  (from the title bar's template menu). */
  only?: "installed";
  /** Shown at startup: adds Open, recent files and the example report. */
  start?: boolean;
  recentFiles?: string[];
  /** Opens a document: the dialog without a path, else that recent file. */
  onOpen?: (path?: string) => void;
  onExample?: () => void;
  onExamplePresentation?: () => void;
  onClose: () => void;
}) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const [query, setQuery] = useState("");
  const tex = only === "installed";
  const shown = tex ? [] : builtins.filter((b) => matches(query, b.name, b.description, b.class));
  const groups = [
    { title: "Reports", icon: <GraduationCap size={15} />, items: shown.filter((b) => !b.class && b.kind !== "presentation") },
    { title: "Presentations", icon: <Presentation size={15} />, items: shown.filter((b) => b.kind === "presentation") },
    { title: "Journals and conferences", icon: <BookOpen size={15} />, items: shown.filter((b) => b.class) },
  ];
  const users = tex ? [] : userTemplates.filter((u) => matches(query, u.name, u.description, u.class));
  // Built-in templates already cover some installed classes (IEEEtran, acmart…).
  const builtinClasses = useMemo(() => new Set(builtins.map((b) => b.class).filter(Boolean)), [builtins]);
  const fromTex = installed.filter((t) => !builtinClasses.has(t.class) && matches(query, t.name, t.description, t.class, t.package));
  const nothing = query.trim() !== "" && shown.length === 0 && users.length === 0 && fromTex.length === 0;

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        className={`modal gallery${start ? " start" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label={start ? "Start" : tex ? "Templates from your TeX installation" : "New from template"}
      >
        {start && (
          <aside className="start-side">
            <div className="start-brand">
              <strong>Texloom</strong>
              <span className="muted">LaTeX documents, visually</span>
            </div>
            <button className="start-action" onClick={() => onOpen?.()}>
              <FolderOpen size={16} aria-hidden="true" /> Open document…
            </button>
            <button className="start-action" onClick={onExample}>
              <Sparkles size={16} aria-hidden="true" /> Open the example report
            </button>
            <button className="start-action" onClick={onExamplePresentation}>
              <Presentation size={16} aria-hidden="true" /> Open the example presentation
            </button>
            {recentFiles.length > 0 && (
              <>
                <div className="start-heading">Recent</div>
                {recentFiles.map((f) => (
                  <button key={f} className="start-recent" onClick={() => onOpen?.(f)} title={f}>
                    <History size={14} aria-hidden="true" />
                    <span className="start-recent-text">
                      <span>{f.split(/[\\/]/).pop()}</span>
                      <span className="muted">{f.replace(/[\\/][^\\/]*$/, "").replace(/^\/home\/[^/]+/, "~")}</span>
                    </span>
                  </button>
                ))}
              </>
            )}
          </aside>
        )}
        <div className="gallery-main">
        <div className="gallery-head">
          <h2>{start ? "Start a new document" : tex ? "From your TeX installation" : "New from template"}</h2>
          <label className="gallery-search">
            <Search size={14} aria-hidden="true" />
            <input
              type="search"
              placeholder={tex ? `Search ${fromTex.length} classes` : "Search templates"}
              autoFocus={tex}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              aria-label="Search templates"
              spellCheck={false}
            />
          </label>
          <button className="icon-button" onClick={onClose} aria-label="Close" title={start ? "Start with an empty document (Esc)" : "Close (Esc)"}>
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
          {nothing && <p className="muted gallery-empty">No template matches “{query.trim()}”.</p>}
          {!tex && (users.length > 0 || !query.trim()) && (
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
                {users.map((u) => (
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
          )}
          {(fromTex.length > 0 || (tex && !query.trim())) && (
            <section>
              {!tex && (
                <h3>
                  <Library size={15} />
                  From your TeX installation
                  <span className="gallery-count">{fromTex.length}</span>
                </h3>
              )}
              <p className="muted gallery-note">
                Classes found in your TeX distribution, by kind of document, with a standard title block you can adjust under Design.
                {tex && " Choosing one restyles this document; its text stays."}
                {engine === "tectonic" && " They compile with your installed TeX; picking one offers to switch the engine from Tectonic to pdfLaTeX."}
              </p>
              {TEX_GROUPS.map((g) => {
                const items = fromTex.filter((t) => t.category === g.category);
                if (items.length === 0) return null;
                return (
                  <div key={g.category} className="tex-group">
                    <h4>
                      {g.title} <span className="gallery-count">{items.length}</span>
                    </h4>
                    <div className="gallery-grid compact">
                      {items.map((t) => (
                        <button
                          key={t.class}
                          className={`gallery-card${t.class === currentClass ? " current" : ""}`}
                          onClick={() => onPickInstalled?.(t.class)}
                          title={`${t.description} (package ${t.package})`}
                        >
                          <span className="gallery-card-top">
                            <FileText size={16} aria-hidden="true" />
                            <code>{t.class}</code>
                          </span>
                          <strong>{t.name}</strong>
                          {t.name !== t.description && <span className="muted">{t.description}</span>}
                        </button>
                      ))}
                    </div>
                  </div>
                );
              })}
              {tex && !query.trim() && (
                <div className="tex-group">
                  <h4>Presentations</h4>
                  <div className="gallery-grid compact">
                    {builtins
                      .filter((b) => b.kind === "presentation")
                      .map((b) => (
                        <button key={b.id} className={`gallery-card${b.id === currentId ? " current" : ""}`} onClick={() => onPick(b.id)}>
                          <span className="gallery-card-top">
                            <Presentation size={16} aria-hidden="true" />
                            <code>beamer</code>
                          </span>
                          <strong>{b.name}</strong>
                          <span className="muted">
                            {installedThemes > 0
                              ? `With ${installedThemes} beamer themes from your TeX installation in the theme picker (Theme in the toolbar).`
                              : b.description}
                          </span>
                        </button>
                      ))}
                  </div>
                </div>
              )}
              {waiting.length > 0 && !query.trim() && (
                <p className="muted gallery-note tex-waiting">
                  Also installed:{" "}
                  {waiting
                    .filter((w) => WAITING_NAMES[w.category])
                    .map((w) => `${w.count} ${WAITING_NAMES[w.category]} class${w.count === 1 ? "" : "es"}`)
                    .join(", ")}
                  . Texloom will offer them once it can write those kinds of documents.
                </p>
              )}
            </section>
          )}
        </div>
        {!tex && (
        <div className="gallery-foot">
          <button className="tb-button" onClick={onImport}>
            <FileInput size={15} aria-hidden="true" /> Import template…
          </button>
          <span className="muted">A publisher’s LaTeX kit, a template package, or a template file. The template can be changed later from the title bar.</span>
        </div>
        )}
        </div>
      </div>
    </div>
  );
}
