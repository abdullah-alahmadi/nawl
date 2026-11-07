// "New from template": every template in one place — Texloom's own, the user's
// (My templates) and the classes of the installed TeX distribution — filtered by
// category chips (journals, theses, books…), shown as cards or a compact list, with
// the recently used ones first. Choosing one starts an empty document with it. At
// startup it is the start window, with Open, recent files and the examples as well.
import { useContext, useEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from "react";
import { Check, FileInput, FileText, FolderOpen, History, LayoutGrid, List, Presentation, Search, Sparkles, Trash2, X } from "lucide-react";
import type { BuiltinTemplate, InstalledCategory, InstalledList, InstalledTemplate, UserTemplate } from "./api";
import { LanguageContext, translate, useT, type MessageKey } from "./i18n";

// Labels below are message keys, translated when shown.

/** Categories, in the order of the chips. */
type Category = "arabic" | "report" | "presentation" | "journal" | "thesis" | "book" | "general" | "mine";
const CATEGORIES: { key: Category; label: MessageKey }[] = [
  { key: "arabic", label: "dialogs.categoryArabic" },
  { key: "report", label: "dialogs.categoryReports" },
  { key: "presentation", label: "dialogs.categoryPresentations" },
  { key: "journal", label: "dialogs.categoryJournals" },
  { key: "thesis", label: "dialogs.categoryTheses" },
  { key: "book", label: "dialogs.categoryBooks" },
  { key: "general", label: "dialogs.categoryArticles" },
  { key: "mine", label: "dialogs.categoryMine" },
];
const CATEGORY_TITLES: Record<Category, MessageKey> = {
  arabic: "dialogs.categoryTitleArabic",
  report: "dialogs.categoryReports",
  presentation: "dialogs.categoryPresentations",
  journal: "dialogs.categoryTitleJournals",
  thesis: "dialogs.categoryTitleTheses",
  book: "dialogs.categoryBooks",
  general: "dialogs.categoryTitleArticles",
  mine: "dialogs.categoryMine",
};

/** Names for the kinds Texloom can't write yet. */
const WAITING_NAMES: Partial<Record<InstalledCategory, MessageKey>> = {
  letter: "dialogs.waitingLetter",
  cv: "dialogs.waitingCv",
  poster: "dialogs.waitingPoster",
  exam: "dialogs.waitingExam",
  presentation: "dialogs.waitingPresentation",
};

type Source = "builtin" | "tex" | "mine";
const SOURCE_LABELS: Record<Source, MessageKey> = { builtin: "dialogs.sourceBuiltin", tex: "dialogs.sourceTex", mine: "dialogs.sourceMine" };

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
  const t = useT();
  const lang = useContext(LanguageContext);
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
      const cat: Category =
        b.language === "arabic" ? "arabic" : b.kind === "presentation" ? "presentation" : b.class ? "journal" : b.topLevel === "section" ? "general" : "report";
      if (tex && cat !== "presentation") continue;
      out.push({
        key: `builtin:${b.id}`,
        name: b.name,
        description:
          b.kind === "presentation" && installedThemes > 0
            ? `${b.description} ${translate(lang, "dialogs.presentationThemesAdded", { n: installedThemes })}`
            : b.description,
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
    for (const it of installed) {
      if (builtinClasses.has(it.class) || !["journal", "thesis", "book", "general"].includes(it.category)) continue;
      out.push({
        key: `tex:${it.class}`,
        name: it.name,
        description: it.name === it.description ? translate(lang, "dialogs.classAndPackage", { cls: it.class, pkg: it.package }) : it.description,
        cls: it.class,
        source: "tex",
        category: it.category as Category,
        current: it.class === currentClass,
        pick: () => onPickInstalled?.(it.class),
      });
    }
    return out;
  }, [builtins, userTemplates, installed, tex, currentId, currentPath, currentClass, installedThemes, onPick, onPickUser, onRemoveUser, onPickInstalled, lang]);

  const recent = recentTemplates.map((k) => items.find((i) => i.key === k)).filter((i): i is Item => !!i);
  const counts = (c: Category) => items.filter((i) => i.category === c).length;
  const chips = [
    { key: "all", label: t("dialogs.chipAll"), count: items.length },
    ...(recent.length && !tex ? [{ key: "recent", label: t("dialogs.recentlyUsed"), count: recent.length }] : []),
    ...CATEGORIES.filter((c) => !tex || c.key !== "mine").map((c) => ({ key: c.key, label: t(c.label), count: counts(c.key) })),
  ].filter((c) => c.key === "all" || c.key === "mine" || c.count > 0);
  const active = chips.some((c) => c.key === category) ? category : "all";

  // Searching looks through everything, whatever the chip.
  const searching = query.trim() !== "";
  const found = items.filter((i) => matches(query, i.name, i.description, i.cls));
  const sections: { title: string; items: Item[]; key: string }[] = searching
    ? [{ key: "results", title: t("dialogs.matching", { n: found.length }), items: found }]
    : active === "recent"
      ? [{ key: "recent", title: t("dialogs.recentlyUsed"), items: recent }]
      : active === "all"
        ? [
            ...(recent.length && !tex ? [{ key: "recent", title: t("dialogs.recentlyUsed"), items: recent }] : []),
            ...CATEGORIES.filter((c) => !tex || c.key !== "mine").map((c) => ({ key: c.key, title: t(CATEGORY_TITLES[c.key]), items: items.filter((i) => i.category === c.key) })),
          ]
        : [{ key: active, title: t(CATEGORY_TITLES[active as Category]), items: items.filter((i) => i.category === active) }];

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
      <button className="tpl-pick" onClick={i.pick} title={i.cls ? t("dialogs.descriptionWithClass", { description: i.description, cls: i.cls }) : i.description}>
        {shownView === "list" ? (
          <>
            <span className="tpl-row-icon">{i.icon ?? <FileText size={15} aria-hidden="true" />}</span>
            <strong className="tpl-name">{i.name}</strong>
            {i.cls && <code className="tpl-class">{i.cls}</code>}
            <span className="muted tpl-desc">{i.description}</span>
            <span className={`tpl-source tpl-source-${i.source}`}>{t(showCategory ? CATEGORY_TITLES[i.category] : SOURCE_LABELS[i.source])}</span>
            {i.current && <Check size={14} className="tpl-current" aria-label={t("dialogs.currentTemplate")} />}
          </>
        ) : (
          <>
            <span className="gallery-card-top">
              {i.icon ?? <FileText size={17} aria-hidden="true" />}
              {i.cls && <code>{i.cls}</code>}
            </span>
            <strong>{i.name}</strong>
            <span className="muted tpl-desc">{i.description}</span>
            <span className={`tpl-source tpl-source-${i.source}`}>{t(showCategory ? CATEGORY_TITLES[i.category] : SOURCE_LABELS[i.source])}</span>
          </>
        )}
      </button>
      {i.remove && (
        <button className="icon-button small tpl-remove" onClick={i.remove} title={t("dialogs.removeFromMine")} aria-label={t("dialogs.removeNamed", { name: i.name })}>
          <Trash2 size={13} />
        </button>
      )}
    </div>
  );

  const importCard = (
    <button className="gallery-import" onClick={onImport}>
      <FileInput size={20} aria-hidden="true" />
      <span>
        <strong>{t("dialogs.importAnyJournal")}</strong>
        <span className="muted">{t("dialogs.importAnyJournalHint")}</span>
      </span>
    </button>
  );

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div
        className={`modal gallery${start ? " start" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label={start ? t("dialogs.start") : tex ? t("dialogs.templatesFromTex") : t("dialogs.newFromTemplate")}
      >
        {start && (
          <aside className="start-side">
            <div className="start-brand">
              <strong>Texloom</strong>
              <span className="muted">{t("dialogs.tagline")}</span>
            </div>
            <button className="start-action" onClick={() => onOpen?.()}>
              <FolderOpen size={16} aria-hidden="true" /> {t("dialogs.openDocument")}
            </button>
            <button className="start-action" onClick={onExample}>
              <Sparkles size={16} aria-hidden="true" /> {t("dialogs.openExampleReport")}
            </button>
            <button className="start-action" onClick={onExamplePresentation}>
              <Presentation size={16} aria-hidden="true" /> {t("dialogs.openExamplePresentation")}
            </button>
            {recentFiles.length > 0 && (
              <>
                <div className="start-heading">{t("dialogs.recent")}</div>
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
            <h2>{start ? t("dialogs.startNewDocument") : tex ? t("dialogs.fromTex") : t("dialogs.newFromTemplate")}</h2>
            <label className="gallery-search">
              <Search size={14} aria-hidden="true" />
              <input
                type="search"
                placeholder={t("dialogs.searchCountTemplates", { n: items.length })}
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
                aria-label={t("dialogs.searchTemplates")}
                spellCheck={false}
              />
            </label>
            <span className="segmented gallery-view" role="radiogroup" aria-label={t("dialogs.view")}>
              <button role="radio" aria-checked={shownView === "grid"} className={shownView === "grid" ? "active" : ""} onClick={() => setView("grid")} title={t("dialogs.viewCards")}>
                <LayoutGrid size={14} aria-hidden="true" />
              </button>
              <button role="radio" aria-checked={shownView === "list"} className={shownView === "list" ? "active" : ""} onClick={() => setView("list")} title={t("dialogs.viewList")}>
                <List size={14} aria-hidden="true" />
              </button>
            </span>
            <button className="icon-button" onClick={onClose} aria-label={t("common.close")} title={start ? t("dialogs.startEmpty") : t("dialogs.closeEsc")}>
              <X size={17} />
            </button>
          </div>
          <div className="gallery-chips" role="tablist" aria-label={t("dialogs.categories")}>
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
                    <p className="muted gallery-note">{t("dialogs.beamerThemesNote")}</p>
                  )}
                  {sec.items.length > 0 ? (
                    <div className={shownView === "list" ? "tpl-list" : "gallery-grid tpl-grid"}>{sec.items.map((i) => card(i, showCategory))}</div>
                  ) : searching ? (
                    <p className="muted gallery-empty">{t("dialogs.noTemplateMatches", { query: query.trim() })}</p>
                  ) : sec.key === "mine" ? null : (
                    <p className="muted gallery-empty">{t("dialogs.noTemplatesInCategory")}</p>
                  )}
                  {sec.key === "mine" && !searching && importCard}
                </section>
              );
            })}
            {!searching && (active === "all" || ["journal", "thesis", "book", "general"].includes(active)) && installed.length > 0 && (
              <p className="muted gallery-note tex-waiting">
                {t("dialogs.texNote")}
                {tex && t("dialogs.texNoteRestyle")}
                {engine === "tectonic" && t("dialogs.texNoteTectonic")}
                {waiting.length > 0 &&
                  t("dialogs.texNoteWaiting", {
                    list: waiting
                      .filter((w) => WAITING_NAMES[w.category])
                      .map((w) => t(w.count === 1 ? "dialogs.waitingItemOne" : "dialogs.waitingItemMany", { n: w.count, kind: t(WAITING_NAMES[w.category]!) }))
                      .join(t("dialogs.listSeparator")),
                  })}
              </p>
            )}
          </div>
          {!tex && (
            <div className="gallery-foot">
              <button className="tb-button" onClick={onImport}>
                <FileInput size={15} aria-hidden="true" /> {t("dialogs.importTemplate")}
              </button>
              <span className="muted">{t("dialogs.importTemplateHint")}</span>
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
