// "New from template": every template in one place — Texloom's own, the user's
// (My templates) and the classes of the installed TeX distribution — filtered by
// category chips (journals, theses, books…), shown as cards or a compact list, with
// the recently used ones first. Choosing one starts an empty document with it. At
// startup it is the start window, with Open, recent files and the examples as well.
import { useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import { Check, FileInput, FileText, FolderOpen, History, LayoutGrid, List, Presentation, Search, Sparkles, Trash2, X } from "lucide-react";
import type { BuiltinTemplate, InstalledCategory, InstalledList, InstalledTemplate, UserTemplate } from "./api";

/** Categories, in the order of the chips. */
type Category = "report" | "presentation" | "journal" | "thesis" | "book" | "general" | "mine";
const CATEGORIES: { key: Category; label: string }[] = [
  { key: "report", label: "Reports" },
  { key: "presentation", label: "Presentations" },
  { key: "journal", label: "Journals" },
  { key: "thesis", label: "Theses" },
  { key: "book", label: "Books" },
  { key: "general", label: "Articles" },
  { key: "mine", label: "My templates" },
];
const CATEGORY_TITLES: Record<Category, string> = {
  report: "Reports",
  presentation: "Presentations",
  journal: "Journals and conferences",
  thesis: "Theses and dissertations",
  book: "Books",
  general: "Articles and reports",
  mine: "My templates",
};

/** Names for the kinds Texloom can't write yet. */
const WAITING_NAMES: Partial<Record<InstalledCategory, string>> = {
  letter: "letter",
  cv: "CV",
  poster: "poster",
  exam: "exam and exercise",
  presentation: "presentation (other than beamer)",
};

type Source = "builtin" | "tex" | "mine";
const SOURCE_LABELS: Record<Source, string> = { builtin: "Built in", tex: "TeX installation", mine: "My template" };

/** One template, wherever it comes from. */
interface Item {
  /** "builtin:id", "tex:class" or "mine:path": also what "recently used" remembers. */
  key: string;
  name: string;
  description: string;
  cls: string | null;
  source: Source;
  category: Category;
  current: boolean;
  icon?: ReactNode;
  pick: () => void;
  remove?: () => void;
}

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
  view = "grid",
  category: savedCategory = "all",
  recentTemplates = [],
  onPrefs,
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
  /** Cards or a compact list (remembered in the settings). */
  view?: "grid" | "list";
  /** The chosen category (remembered in the settings). */
  category?: string;
  /** Keys of the templates documents were last started with. */
  recentTemplates?: string[];
  onPrefs?: (p: { galleryView?: "grid" | "list"; galleryCategory?: string }) => void;
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
  // The TeX installation window has its own category, not the remembered one.
  const [texCategory, setTexCategory] = useState("all");
  const category = tex ? texCategory : savedCategory;
  const setCategory = (c: string) => (tex ? setTexCategory(c) : onPrefs?.({ galleryCategory: c }));
  const [localView, setLocalView] = useState(view);
  const shownView = onPrefs ? view : localView;
  const setView = (v: "grid" | "list") => (onPrefs ? onPrefs({ galleryView: v }) : setLocalView(v));

  // Every template as one list. Built-in templates already cover some installed
  // classes (IEEEtran, acmart…): those aren't listed twice.
  const items = useMemo<Item[]>(() => {
    const out: Item[] = [];
    const builtinClasses = new Set(builtins.map((b) => b.class).filter(Boolean));
    for (const b of builtins) {
      const cat: Category = b.kind === "presentation" ? "presentation" : b.class ? "journal" : "report";
      if (tex && cat !== "presentation") continue;
      out.push({
        key: `builtin:${b.id}`,
        name: b.name,
        description: b.kind === "presentation" && installedThemes > 0 ? `${b.description} Your TeX installation adds ${installedThemes} themes to its theme picker.` : b.description,
        cls: b.class ?? (b.kind === "presentation" ? "beamer" : null),
        source: "builtin",
        category: cat,
        current: b.id === currentId,
        icon: b.kind === "presentation" ? <Presentation size={16} aria-hidden="true" /> : undefined,
        pick: () => onPick(b.id),
      });
    }
    if (!tex)
      for (const u of userTemplates)
        out.push({
          key: `mine:${u.path}`,
          name: u.name,
          description: u.description,
          cls: u.class,
          source: "mine",
          category: "mine",
          current: u.path === currentPath,
          pick: () => onPickUser(u),
          remove: () => onRemoveUser(u),
        });
    for (const t of installed) {
      if (builtinClasses.has(t.class) || !["journal", "thesis", "book", "general"].includes(t.category)) continue;
      out.push({
        key: `tex:${t.class}`,
        name: t.name,
        description: t.name === t.description ? `Class ${t.class} (package ${t.package})` : t.description,
        cls: t.class,
        source: "tex",
        category: t.category as Category,
        current: t.class === currentClass,
        pick: () => onPickInstalled?.(t.class),
      });
    }
    return out;
  }, [builtins, userTemplates, installed, tex, currentId, currentPath, currentClass, installedThemes, onPick, onPickUser, onRemoveUser, onPickInstalled]);

  const recent = recentTemplates.map((k) => items.find((i) => i.key === k)).filter((i): i is Item => !!i);
  const counts = (c: Category) => items.filter((i) => i.category === c).length;
  const chips = [
    { key: "all", label: "All", count: items.length },
    ...(recent.length && !tex ? [{ key: "recent", label: "Recently used", count: recent.length }] : []),
    ...CATEGORIES.filter((c) => !tex || c.key !== "mine").map((c) => ({ key: c.key, label: c.label, count: counts(c.key) })),
  ].filter((c) => c.key === "all" || c.key === "mine" || c.count > 0);
  const active = chips.some((c) => c.key === category) ? category : "all";

  // Searching looks through everything, whatever the chip.
  const searching = query.trim() !== "";
  const found = items.filter((i) => matches(query, i.name, i.description, i.cls));
  const sections: { title: string; items: Item[]; key: string }[] = searching
    ? [{ key: "results", title: `${found.length} matching`, items: found }]
    : active === "recent"
      ? [{ key: "recent", title: "Recently used", items: recent }]
      : active === "all"
        ? [
            ...(recent.length && !tex ? [{ key: "recent", title: "Recently used", items: recent }] : []),
            ...CATEGORIES.filter((c) => !tex || c.key !== "mine").map((c) => ({ key: c.key, title: CATEGORY_TITLES[c.key], items: items.filter((i) => i.category === c.key) })),
          ]
        : [{ key: active, title: CATEGORY_TITLES[active as Category], items: items.filter((i) => i.category === active) }];

  // Arrow keys move between templates; Enter (a button's own) picks.
  const body = useRef<HTMLDivElement>(null);
  const onBodyKey = (e: ReactKeyboardEvent) => {
    if (!["ArrowDown", "ArrowUp", "ArrowLeft", "ArrowRight"].includes(e.key)) return;
    const all = [...(body.current?.querySelectorAll<HTMLElement>(".tpl-pick") ?? [])];
    const i = all.indexOf(document.activeElement as HTMLElement);
    if (i < 0) return;
    e.preventDefault();
    const step = shownView === "list" || e.key === "ArrowLeft" || e.key === "ArrowRight" ? 1 : columns(body.current);
    const next = e.key === "ArrowDown" || e.key === "ArrowRight" ? i + step : i - step;
    all[Math.max(0, Math.min(all.length - 1, next))]?.focus();
  };

  const card = (i: Item, showCategory: boolean) => (
    <div key={`${i.key}`} className={`tpl ${shownView === "list" ? "tpl-row" : "gallery-card tpl-card"}${i.current ? " current" : ""}`}>
      <button className="tpl-pick" onClick={i.pick} title={i.cls ? `${i.description} (class ${i.cls})` : i.description}>
        {shownView === "list" ? (
          <>
            <span className="tpl-row-icon">{i.icon ?? <FileText size={15} aria-hidden="true" />}</span>
            <strong className="tpl-name">{i.name}</strong>
            {i.cls && <code className="tpl-class">{i.cls}</code>}
            <span className="muted tpl-desc">{i.description}</span>
            <span className={`tpl-source tpl-source-${i.source}`}>{showCategory ? CATEGORY_TITLES[i.category] : SOURCE_LABELS[i.source]}</span>
            {i.current && <Check size={14} className="tpl-current" aria-label="Current template" />}
          </>
        ) : (
          <>
            <span className="gallery-card-top">
              {i.icon ?? <FileText size={17} aria-hidden="true" />}
              {i.cls && <code>{i.cls}</code>}
            </span>
            <strong>{i.name}</strong>
            <span className="muted tpl-desc">{i.description}</span>
            <span className={`tpl-source tpl-source-${i.source}`}>{showCategory ? CATEGORY_TITLES[i.category] : SOURCE_LABELS[i.source]}</span>
          </>
        )}
      </button>
      {i.remove && (
        <button className="icon-button small tpl-remove" onClick={i.remove} title="Remove from My templates" aria-label={`Remove ${i.name}`}>
          <Trash2 size={13} />
        </button>
      )}
    </div>
  );

  const importCard = (
    <button className="gallery-import" onClick={onImport}>
      <FileInput size={20} aria-hidden="true" />
      <span>
        <strong>Import a template for any journal</strong>
        <span className="muted">
          Choose the publisher’s LaTeX kit (.zip from the journal’s author guidelines), a template package someone shared, or a Texloom template (.json).
        </span>
      </span>
    </button>
  );

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
                placeholder={`Search ${items.length} templates`}
                autoFocus={tex}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => {
                  // Down from the search box goes to the first result.
                  if (e.key === "ArrowDown") {
                    e.preventDefault();
                    body.current?.querySelector<HTMLElement>(".tpl-pick")?.focus();
                  }
                }}
                aria-label="Search templates"
                spellCheck={false}
              />
            </label>
            <span className="segmented gallery-view" role="radiogroup" aria-label="View">
              <button role="radio" aria-checked={shownView === "grid"} className={shownView === "grid" ? "active" : ""} onClick={() => setView("grid")} title="Cards">
                <LayoutGrid size={14} aria-hidden="true" />
              </button>
              <button role="radio" aria-checked={shownView === "list"} className={shownView === "list" ? "active" : ""} onClick={() => setView("list")} title="List">
                <List size={14} aria-hidden="true" />
              </button>
            </span>
            <button className="icon-button" onClick={onClose} aria-label="Close" title={start ? "Start with an empty document (Esc)" : "Close (Esc)"}>
              <X size={17} />
            </button>
          </div>
          <div className="gallery-chips" role="tablist" aria-label="Categories">
            {chips.map((c) => (
              <button
                key={c.key}
                role="tab"
                aria-selected={!searching && active === c.key}
                className={`chip${!searching && active === c.key ? " active" : ""}`}
                onClick={() => {
                  setQuery("");
                  setCategory(c.key);
                }}
              >
                {c.label}
                <span className="chip-count">{c.count}</span>
              </button>
            ))}
          </div>
          <div className="gallery-body" ref={body} onKeyDown={onBodyKey}>
            {sections.map((sec) => {
              const showCategory = searching || sec.key === "recent";
              // Sections of "All" without templates are left out (My templates keeps its import card).
              if (sec.items.length === 0 && active === "all" && !searching && sec.key !== "mine") return null;
              return (
                <section key={sec.key}>
                  {(active === "all" || searching) && <h3>{sec.title}</h3>}
                  {sec.key === "presentation" && tex && installedThemes > 0 && (
                    <p className="muted gallery-note">Beamer themes from your TeX installation are in the theme picker (Theme in the toolbar).</p>
                  )}
                  {sec.items.length > 0 ? (
                    <div className={shownView === "list" ? "tpl-list" : "gallery-grid tpl-grid"}>{sec.items.map((i) => card(i, showCategory))}</div>
                  ) : searching ? (
                    <p className="muted gallery-empty">No template matches “{query.trim()}”.</p>
                  ) : sec.key === "mine" ? null : (
                    <p className="muted gallery-empty">No templates in this category.</p>
                  )}
                  {sec.key === "mine" && !searching && importCard}
                </section>
              );
            })}
            {!searching && (active === "all" || ["journal", "thesis", "book", "general"].includes(active)) && installed.length > 0 && (
              <p className="muted gallery-note tex-waiting">
                “TeX installation” templates use the classes installed on this computer, with a standard title block you can adjust under Design.
                {tex && " Choosing one restyles this document; its text stays."}
                {engine === "tectonic" && " They compile with your installed TeX; picking one offers to switch the engine from Tectonic to pdfLaTeX."}
                {waiting.length > 0 &&
                  ` Also installed: ${waiting
                    .filter((w) => WAITING_NAMES[w.category])
                    .map((w) => `${w.count} ${WAITING_NAMES[w.category]} class${w.count === 1 ? "" : "es"}`)
                    .join(", ")}; Texloom will offer them once it can write those kinds of documents.`}
              </p>
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

/** How many cards fit in a row of the grid (for arrow keys up and down). */
function columns(body: HTMLElement | null): number {
  const grid = body?.querySelector<HTMLElement>(".tpl-grid");
  if (!grid) return 1;
  return Math.max(1, getComputedStyle(grid).gridTemplateColumns.split(" ").length);
}
