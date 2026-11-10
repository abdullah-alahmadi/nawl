# Texloom plan: visual LaTeX editor with a no-code template designer

A LyX-like structured editor built on Tauri. Its main difference from LyX: **templates
(document classes) are designed visually**, with no code. The north-star demo is a
university senior design report template.

## Core idea

A template is **structured data** edited in a visual designer, and it **compiles into a
`.cls` file**. We never try to turn arbitrary LaTeX back into visual controls. The same
template data also generates **CSS**, so the editor approximates the final look, while a
real PDF preview (Tectonic) shows the exact output.

## Tech stack

| Layer | Choice |
|---|---|
| App shell | Tauri 2 (Rust backend, system webview) |
| Frontend | TypeScript + React + Vite |
| Editor | TipTap (ProseMirror) with a custom schema |
| Math | MathLive |
| PDF preview | pdf.js |
| Rust ↔ TS types | ts-rs (`cargo test` writes `src/bindings/`) |
| TeX engine | Tectonic, bundled as a sidecar binary (may move to the crate later) |
| Bibliography | BibTeX, run automatically by Tectonic (IEEEtran / plain / natbib + apalike); `biblatex` crate parses entries for the editor |
| Codegen | Plain Rust string building (Jinja-style `{{ }}`/`{% %}` collide with TeX braces and `%` comments) |
| Tests | insta (snapshots of generated LaTeX), Playwright (UI) |

## Repository layout

```
crates/
  core/      texloom-core: Document + Template models, placeholders, validation. No UI, no TeX.
  codegen/   texloom-codegen: Template → .cls, Document → .tex
  compile/   texloom-compile: Tectonic runner, log parsing → friendly errors
  cli/       texloom-cli: `render doc.loom -t template.json`, `export`, `check`
templates/   built-in templates (senior-design/template.json is the reference)
examples/    sample documents
src-tauri/   Tauri app: commands that call the crates
src/         React frontend (Tauri's default layout keeps it at the repo root)
  editor/    TipTap schema + custom nodes (equation, figure, citation, ref)
  designer/  template designer panels + title-page canvas
  preview/   pdf.js pane
scripts/     fetch-tectonic.sh (sidecar binary), copy-mathlive-assets.mjs (postinstall)
```

## File formats

- **Document** (`.loom`): currently JSON `{"format": "texloom", "version": 1, "metadata": {...}, "doc": <ProseMirror JSON>}`.
  The `doc` JSON *is* the typed model (`texloom_core::Body`), so no conversion layer exists.
  It embeds a copy of the template. **Save as single file…** writes the same JSON as
  `document.loom` inside a zip with the images (`figures/…`); opening detects either.
- **Template**: currently `template.json` with assets resolved relative to its folder.
  Later (zip): `template.json` + `assets/` (logos, fonts).
- **Export** (zip): `report.tex` + `<template>.cls` + assets + bibliography. Compiles
  anywhere, including Overleaf.

### Template model

See `crates/core/src/template.rs`. In short:

```rust
pub struct Template {
    pub meta: TemplateMeta,               // id doubles as the .cls name
    pub page: PageSetup,                  // paper, margins (mm), two-sided, line spacing, paragraphs
    pub fonts: Fonts,                     // base size + main/sans/mono (TeX Gyre built-ins or system)
    pub structure: Structure,             // chapters vs sections, numbering depth, page number styles
    pub headings: Headings,               // 4 levels: size, weight, case, align, label "Chapter {n}", spacing
    pub header_footer: HeaderFooter,      // front matter / main / chapter-opening L/C/R slots
    pub title_page: TitlePage,            // text/image/rule items placed in mm
    pub fields: Vec<MetadataField>,       // title, team (list), abstract (paragraphs)…
    pub front_matter: Vec<FrontMatterItem>, // title page, field pages (abstract), TOC, LoF, LoT
    pub raw_preamble: Option<String>,     // escape hatch
}
```

Placeholders like `{title}`, `{page}`, `{pages}`, `{chapter}` work in header/footer
slots and title-page text; `{n}` in heading labels. Templates are validated with paths
to each problem (`headerFooter.main.header.right: unknown placeholder {nope}`).

The generated `.cls` is standalone: documents set `\texloomfield{id}{value}` and call
`\texloomfrontmatter` / `\texloommainmatter`. Package mapping: geometry, fontspec,
setspace, titlesec, fancyhdr + lastpage, TikZ overlay (title page), hyperref. Still to
map: caption (captions), tocloft (TOC styling), bibliography.

## Phases

### Phase 0: Prove the risky parts first (~1–2 weeks)
- [x] Tauri 2 + React + TipTap app; save/load JSON through a Rust command
      (Phase 0 saves plain JSON `.loom`; the zip format comes in Phase 2)
- [x] Compile with the Tectonic sidecar; show the PDF in pdf.js (debounced, stale results dropped)
- [x] MathLive equation as a custom TipTap node storing LaTeX (`$$ ` shortcut, toolbar button)
- [x] Throwaway `codegen` crate: ProseMirror JSON → LaTeX (replaced by typed model in Phase 1)
- [ ] Check behaviour on Linux (WebKitGTK), Windows and macOS
- Known rough edges: typing immediately after `$$ ` can land outside the new equation;
  Ctrl+Z inside an equation goes to MathLive's own undo, not the document's

### Phase 1: Core model and LaTeX generation, no UI (~2–3 weeks)
- [x] `core` structs + TS type export; template + metadata validation
- [x] `codegen`: Template → .cls, Document → .tex; correct escaping of special characters
- [x] `compile`: Tectonic runner + readable errors
- [x] `cli` + snapshot tests; hand-written `senior-design` reference template + sample report
- [x] App uses the new pipeline with the built-in template; basic "Document info" form
      (generated from the template's fields), metadata saved in `.loom` files
- [x] App: open a `template.json` (Template menu or `TEXLOOM_TEMPLATE`), live reload on
      save, template errors shown while the last valid version stays active; documents
      save their template path + an embedded copy and reopen with it
- Not yet: figures/tables/captions (Phase 2), mirrored headers for two-sided printing

### Phase 2: Editor MVP (~3–4 weeks)
- [x] Figures (image + caption, width slider, replace image) and captioned tables
      (row/column/header controls); numbered like the PDF ("Figure 3.1", "Table 3.1")
- [x] Images live in `figures/` next to the document (a temporary folder before the
      first save); Save As copies them; 16-bit PNGs are converted to 8-bit on import
- [x] Outline sidebar: headings, figures and tables with their numbers; click to jump
- [x] Editor page styled from the template: page size and margins, fonts, spacing,
      heading styles with numbers, caption labels, equation numbers
- [x] Template designer: Captions panel (labels, separator, bold, size, alignment)
- [x] Export PDF; Export .tex writes images too; documents are checked when opened
- [ ] New/blank document command
- [x] Table cells spanning rows (Merge/Split; \multirow, \cline); image cropping (adjustbox trim)
- [x] Find and replace (Ctrl+F / Ctrl+H) with match marks and one-step Replace all
- [x] Single-file .loom (zip with the images) via Save as single file…; plain .loom stays the default

### Phase 3: Template designer v1, styles (~3–4 weeks)
- [x] Template mode with panels: info, page, fonts (built-in + system), structure &
      numbering, headings (with instant on-screen sample), headers/footers (placeholder
      chips), fields, front matter
- [x] Live preview: debounced validation → PDF of the current document; errors shown
      inline and per section while the last valid version stays in the preview
- [x] Save / Save as (copies image assets) / New copy; Ctrl+S saves the template in Template mode
- [x] Save as… asks for a name and saves into My templates (assets and kit files copied), so
      the template is in New from template… right away; "Save to file…" keeps a loose .json
- [x] Instant page preview in the designer (Page / PDF switch): the document styled with
      the draft template on every change
- [x] New blank template (report, article or presentation; A4 or Letter) saved in My templates
- [ ] Undo/redo inside the designer; caption and TOC styling once figures exist

### Phase 4: Title-page designer (~2–3 weeks)
- [x] Page-sized canvas in mm: drag to move, right-edge handle to resize, snap to the
      page centre, arrow-key nudging (Shift ×10, Alt 0.1 mm), anchor picker, Centre button
- [x] Text with `{field}` placeholders (shown with the open document's values), images
      (imported into the template's `assets/`), lines → TikZ overlay
- [x] Canvas geometry matches the PDF (checked against the PDF's word positions: within
      ~0.5 mm; line spacing follows the template's setspace factor)
- [x] Extra fixed pages built the same way (approval/signature, declaration): front-matter "fixed page"
- [x] Snapping to other items / margins; multi-select (Shift-click, frame), align and spread
- [ ] Undo on the title page canvas

### Phase 5: Structure and academic features (~3–4 weeks)
- [x] Chapter skeleton in the template (designer "Chapter list"); New document starts
      from it; missing required chapters are flagged; metadata form from `fields`
- [x] Front matter, TOC/LoF/LoT, roman → arabic page numbering (Phases 1–2)
- [x] Cross-references with stable ids (headings, figures, tables, equations): picker,
      on-screen numbers matching the PDF, broken references shown as "??" and flagged
- [x] Citations from BibTeX: References manager (import/paste .bib), citation picker,
      IEEE / numeric / author–year styles, reference list before the appendices;
      a form to add and edit references (per-type fields, authors, automatic keys)
- [x] Appendices marker: chapters lettered A, B… with their own label ("Appendix {n}")
- [x] Document checks shown in the app: required fields and chapters, broken
      references, citations missing from the references
- [x] Inline math ($…$ input rule, Ctrl+M, toolbar) in paragraphs, captions, cells,
      headings; display equations can be unnumbered (equation*) or multi-line
      (aligned inside equation: one number per block, as on screen)
- [x] Unnumbered headings (Acknowledgment, Preface): `\chapter*` still listed in the
      table of contents and the running header; skipped by numbering and references
- [x] Raw LaTeX block (**TeX**), written as typed: the escape hatch for TikZ etc.
- [ ] Locked sections; cross-reference text options ("Fig. 3" vs "3"); citation page numbers;
      List of Symbols
- [ ] **Milestone:** full senior design report from a template made entirely in the designer
      (all pieces exist; needs an end-to-end walkthrough in the native app)

### Journal and conference templates
- [x] Templates can be built on a publisher's official class (`base`): wrapper `.cls`,
      publisher-specific title block (IEEE journal/conference, Elsevier, ACM, LNCS,
      REVTeX), the publisher's BibTeX style; designer hides class-defined panels
- [x] Built-in: IEEE Journal, IEEE Conference, Elsevier, ACM, Springer LNCS, APS;
      each compiles the sample article (tests/journals.rs)
- [x] Bring your own class: import a publisher kit (.cls/.sty/.bst/logos) into a
      template; kit files are placed next to the document when compiling
- [x] Structured authors (several affiliations, e-mail, ORCID, membership,
      corresponding author, biography + photo) with an author editor; old
      "Name | Affiliation | Email" lines still work
- [x] IEEE Access with IEEE's own kit (removed as a built-in; still works imported: font maps and a spot-colour
      stand-in make the pdfLaTeX kit work under XeTeX; output matches IEEE's sample)
- [x] Author biographies (IEEEbiography, with or without photo); `\EOD` for IEEE Access
- [x] Custom title-block recipes (`{title}`, `{#authors|sep}…{/authors}`, `{?x}…{/x}`)
      plus an optional end block, for publishers without a built-in style
- [x] Typographic punctuation and Latin-1 symbols written as TeX commands, so they
      render in old 8-bit publisher fonts
- [x] Installed TeX classes grouped by kind from Catalogue topics: journals and
      conferences, theses, books (chapters), articles and reports; letters, CVs,
      posters, exams and other presentation classes counted, offered once supported
- [x] Journal templates from the installed TeX: classes TeX Live lists under the
      Catalogue's "journalpub" topic appear in the gallery (with search); tuned title
      blocks for MNRAS, ACS, AAS, OUP, Annals of Mathematics…; title blocks may start
      in the preamble; empty BibTeX style = the class sets it; 39 of 65 classes on a
      full TeX Live compile the sample article (the rest are skipped)
- [ ] Two-column preview in the editor;
      more pdfLaTeX-only kits as they come up (each needs a small stand-in)

### Other document kinds
- [x] Template `kind` (article or presentation); presentations get `presentation`
      settings (beamer theme, colour/font theme, aspect ratio, size, outline slides)
- [x] Presentations: slide, slide title, columns, boxes (block/alert/example) and
      pauses in the model, codegen and editor; slides shown down the page at their
      real shape, listed in the Outline; sections between slides; title slide from fields;
      references on their own slides; built-in "Presentation (Beamer)" + example deck;
      a deck under an article template degrades to plain LaTeX
- [x] Theme picker with previews rendered by the real themes (cached per engine),
      installed TeX Live beamer themes and colour themes, a custom main colour
- [x] Logo (image field in Document info; every slide / title slide / off), short
      title, footer (theme's own / custom left-centre-right with placeholders / none),
      shown on the slides in the editor too
- [x] Articles in Texloom's own layout: a title block on page 1 that flows (rows placed
      below the real bottom of the row above), the abstract under it, one or two
      columns (title block spanning); built-in Article, Two-column Article, Short Paper
- [ ] Posters (tikzposter / beamerposter): a big canvas of boxes in columns
- [ ] Letters (letter / scrlttr2): sender, recipient, opening, closing
- [ ] CVs (moderncv): sections of dated entries
- [ ] Books and theses: parts, front/back matter on top of the report template
- [ ] Presentations next: slide layouts, speaker notes, overlays beyond \pause,
      reordering slides by dragging

### Arabic publishing
Goal: Arabic journals, universities and publishers write in Texloom instead of Word.
Word is an export target only; Texloom never imports Word files.
- [x] Language in the model: template language (main, second, digits), Arabic font,
      Amiri built in, paragraph and heading direction (`dir`)
- [x] LaTeX: polyglossia + bidi block, automatic `\textenglish`/`\textarabic` runs,
      localised built-in words, mirrored layout, Arabic-Indic numbering; the app's own
      reference formatter for documents with Arabic (IEEE, numeric, author-year);
      pdfLaTeX refused with an explanation; RTL slides through beamer-rl (LuaLaTeX)
- [x] Editor: right-to-left pages, Ctrl+Shift+X per paragraph, logical CSS, Arabic
      digits and words on screen, Windows fonts in the font list
- [x] Designer Language section; Arabic journal article, thesis, book and presentation
      templates; Arabic example article
- [x] Word export (`crates/docx`): hand-written OOXML with RTL properties, styles from
      the template, MathML → OMML equations, merged cells, crops, TOC and STYLEREF fields
- [x] Paragraph alignment (left, centre, right as seen on the page; toolbar buttons, no
      shortcuts: the usual ones are taken) for signatures and centred lines, in LaTeX
      and Word; table cells keep theirs. Administrative order template: logo,
      attachments, double rule, address footer; its example signs on the left
- [x] Arabic interface (`src/i18n/`, Settings → Appearance → Language)
- [x] Separate Arabic and foreign reference lists; Arabic sorting without ال
- [x] Date fields with Hijri dates (Umm al-Qura) for {field_hijri}
- [x] Find and replace ignoring Arabic diacritics and letter variants
- [x] Abjad letters for appendices and sub-lists (editor, PDF, Word); Arabic punctuation
      as you type; equation numbers on the left in RTL
- [x] Authors' English names and affiliations; backend messages (errors, validation) in
      Arabic (`texloom_core::tr!`)
- [x] Arabic letters and words in formulas (Arabic font, PDF and Word); Arabic-Indic
      digits in formulas
- [ ] Arabic school notation: Arabic variables and function names (جا، جتا، نها) in the
      editor; fully right-to-left mirrored formulas (LuaLaTeX only)
- [x] Arabic font library: 31 OFL families shipped with the app (catalog in
      fonts/catalog.json), shown in a gallery, sent with the document, embedded in Word
- [ ] Kashida justification (LuaLaTeX only)
- [ ] Exact house styles of particular Arabic journals and universities
  - [x] Majmaah University, College of Engineering graduation project report: cover
        pages before chapters, references and appendices (`structure.coverPages`), and
        title-page images taken from a document's image field (`"asset": "{logo}"`)
- [ ] Arabic-Indic page numbers in Word (Word
      shows them only with its own "Hindi digits" option)
- [ ] More languages on the same model (Persian, Urdu, Hebrew)

### Projects: books and theses as one file per chapter (in progress)

**Why.** Long documents are easier to organize as chapters: each opens on its own, can
be reordered, backed up or tracked in git, and two people can work on different
chapters. Writing gets faster as a side effect: the editor holds one chapter (the
1000-page test book opens in about 7 s as one file; a chapter opens at once), and
LaTeX can recompile just the chapter being written. This is the same idea as LaTeX's
`\include` and LyX's master/child documents.

**Not a replacement for single files.** A `.loom` stays the default. A project is for
documents that have chapters (book, thesis, long report). The templates are the same.

#### File format

A project is a folder:

```
my-thesis/
  my-thesis.loomproj      project file
  chapters/
    01-introduction.loom  one chapter per file
    02-literature.loom
    a1-survey.loom        appendix
  figures/                shared by all chapters
```

- **Project file** (`.loomproj`, JSON):
  `{"format": "texloom-project", "version": 1, "template": {...}, "metadata": {...},
  "bibliography": "...", "chapters": [{"part": "الباب الأول: الإطار النظري"},
  {"file": "chapters/01-introduction.loom"}, …, {"file": "chapters/a1-survey.loom",
  "appendix": true}]}`.
  Everything the whole book shares lives here: the template (embedded, as in a
  `.loom`), title-page metadata, front matter fields, the bibliography and the
  chapter order.
  - **Parts** ("الباب الأول") are entries in the chapter list, not files: a title,
    followed by the chapters it groups until the next part. Books without parts don't
    have any.
  - **Front matter written as text** (abstract, dedication, acknowledgments, الإهداء,
    الشكر والتقدير) stays as the template's metadata fields, edited in Document
    info, as for single documents.
- **Chapter file**: an ordinary `.loom` (`format`, `version`, `doc`) with
  `"project": "../my-thesis.loomproj"` and no template, metadata or bibliography of its
  own. It starts with its chapter heading.
  - A chapter opened on its own (double-clicked) finds its project and opens the
    project at that chapter.
  - If the project file is missing, the chapter opens as a single document with the
    default template, with a notice.
- **Paths** in the project file are relative, so the folder can be moved, zipped or
  synced. "Save as single file…" zips the folder (`.loomproj` inside; opening it
  detects either kind).

#### Writing

- **Chapters panel:** the side rail gets a Chapters tab, listing chapters by their
  titles (from each file's first heading), grouped under their parts, with appendices
  after a separator. From there you can:
  - add a new chapter, or an existing `.loom` as a chapter;
  - add, rename or remove a part (removing a part keeps its chapters);
  - rename (the heading and the file name);
  - drag to reorder;
  - remove (from the project only; the file stays);
  - open in the editor.
- **One chapter in the editor at a time.** Switching chapters saves the current one
  (or keeps it unsaved, the way tabs do).
  - Undo history is per chapter.
  - The Outline shows the open chapter's headings, plus the other chapters' titles
    collapsed, so you can jump anywhere.
- **Document info** edits the project's metadata. References edits the project's
  bibliography.
- **Word count:** the status bar shows the chapter and the whole book ("2,140 / 61,300
  words").
- **Search and replace:** "this chapter" by default, "whole book" as an option. Results
  from other chapters open that chapter at the match.
- **Checks** run per chapter, plus the cross-chapter references (below).

#### Numbering, references and citations across files

The single-document code works on one ProseMirror document (`computeNumbering`,
`citationLabels`). A project adds a **project index**, built by the backend from all
chapter files when the project opens and updated when a chapter is saved or switched.
For each chapter it records:

- its numbered targets: heading, figure, table and equation ids, with their kind, level
  and caption text;
- its citation keys, in order;
- its word count.

The editor uses the index to:
- **start numbering where the previous chapters end.** Chapter 3's first figure is
  "Figure ٣.١"; continuous numbering styles count from the totals so far.
  `computeNumbering` takes a starting state (chapter number, appendix letter, counters
  carried over) instead of always starting from zero.
- **resolve references to other chapters.** A reference whose target isn't in the open
  chapter is looked up in the index; it shows its number, and Ctrl-click opens that
  chapter at the target. Inserting a cross-reference lists targets from every
  chapter, grouped by chapter.
- **number citations in book order.** IEEE numbers by first citation in the whole book:
  citations in earlier chapters come first. The index provides the key order before
  the open chapter.
- **keep ids unique across the project.** The id plugin (`TexloomIds`) also checks the
  index, so copying a figure from one chapter to another gets a new id instead of a
  duplicate.

#### Compiling and exporting

- **Whole book:** codegen writes `main.tex` (preamble, title page, contents, front
  matter) and one `.tex` per chapter, joined with `\include{chapters/…}`. Appendices
  come after `\appendix`, as now. Parts are `\part{…}` lines in `main.tex`.
- **Part style:** templates gain a part heading style in the designer, next to the
  chapter style:
  - a page of its own or not;
  - the label, with Arabic ordinal words ("الباب الأول", "الباب الثاني") or numbers;
  - font and size;
  - whether parts appear in the contents.
  Templates without one get a plain default. Single documents don't use it.
  - Each chapter's `.tex` comes from the existing body generator. `generate_for` gains
    a project entry point that takes the template, metadata, bibliography and the
    ordered chapter bodies.
  - The reference list Texloom writes itself (`refs.rs`, Arabic documents) collects
    citations from all chapters.
- **Just this chapter** (the default while writing a long book): the same files with
  `\includeonly{chapters/03-…}`.
  - LaTeX keeps the other chapters' `.aux` files from the last full compile, so page
    numbers, references and citations to other chapters stay right.
  - **The app compiles the whole book instead, automatically,** whenever those files
    could be out of date:
    - the first compile of a project;
    - after chapters or parts are added, removed or reordered;
    - after a numbered target is added or removed in another chapter, or a citation
      that changes the book's citation order;
    - after the template or bibliography changes.
    The project index tells which of these happened since the last full compile. Once
    the whole book is compiled, compiles go back to just the open chapter.
  - A toolbar switch picks "this chapter" or "whole book". Compile after saving and
    compile while typing follow it.
- **SyncTeX** maps PDF positions back to chapter files: codegen records each block's
  `.tex` file and line, not only its line.
- **Export** (zip) contains `main.tex`, the chapter `.tex` files, the class, figures
  and bibliography. It compiles anywhere, including Overleaf, which handles `\include`.
- **Word export** joins the chapters into one `.docx` (one Word file per book; Word's
  master documents are unreliable).

#### Moving between single files and projects

- **Split into project…** (File menu, for a `.loom` with chapters): writes a project
  folder, one file per top-level chapter heading, splitting at `appendixStart`.
  Figures are copied into `figures/`, and ids are kept so references still work.
- **Join into single document…**: the reverse, for sending one file, or for a project
  that turned out short.
- **New project** in the start window, for the book and thesis templates. It creates
  the folder with the template's sample chapters, or one empty chapter.

#### Recovery, recent files, settings

- **Crash recovery** copies the unsaved chapters and the project file, keyed by the
  project path. On the next start, it offers them back together.
- **Recent files** list projects with their own icon; reopening goes to the last open
  chapter.
- **File watching:** a chapter changed on disk while the project is open (git pull,
  sync) reloads if it has no unsaved changes, otherwise asks. The index is refreshed
  either way.

#### Steps

- [x] Core: `ProjectFile` and `Project` in `texloom-core` (with TypeScript bindings),
      `"project"` in chapter files, validation (unsafe or repeated paths, appendices
      before chapters, empty parts, ids used in two chapters, appendix markers inside
      chapters, chapters without a chapter heading), tests
- [x] Template: part heading style (`headings.part`: label with `{n}`, number in
      words — Arabic ordinals "الباب الأول" / English "Part One" — own page, contents
      line, text style); the class defines `\texloompart` (the base class's `\part`
      without a style; a `\providecommand` fallback for publisher classes); the
      designer's Headings → Part tab; the Arabic book and thesis templates have one
- [x] Codegen: `generate_project_for` (`main.tex` + `chapters/chNN.tex` by position,
      parts, the reference list and `\appendix` in the main file, `\includeonly`),
      each chapter's block lines for SyncTeX; tests (same LaTeX as the joined single
      document; Tectonic and pdfLaTeX compiles of an Arabic and an English project; a
      one-chapter Tectonic compile with `-k` keeps the other chapters' references)
- [x] Compile: `compile_in` builds in a kept folder (only changed files rewritten;
      Tectonic with `--keep-intermediates`), so one-chapter compiles find the other
      chapters' `.aux` files; SyncTeX reads the chapter files too (`read_with`,
      `place_at`, `spot_in`)
- [x] Core: `ProjectIndex` (per chapter: title, part, appendix, words, targets with
      kind/level/numbered/text, citations), with TypeScript bindings
- [x] Backend (`src-tauri/src/project.rs`): `open_project` (template restored,
      unreadable chapters reported), `close_project`, `read_chapter`, `save_chapter`,
      `save_project` (validated, template embedded, unknown fields kept), `new_chapter`,
      `project_index` (with the open chapter's live content), `render_project`
      (build folder per project and engine in the app cache; whole book when the
      signature of template, engine, bibliography, chapter list, targets and citations
      differs from the last full compile; errors located by chapter and block),
      `project_pdf_to_block` / `project_block_to_pdf`, `export_project_tex`,
      `split_into_project`, `join_project`; opening a chapter file reports its project
- [x] Crash recovery of the open chapter (with the project's metadata and
      bibliography), restored into the project at that chapter
- [x] Word export of the whole book (`export_project_docx`; the editor sends MathML for
      every chapter's formulas; parts aren't written)
- [ ] Backend: file watching (chapters changed on disk)
- [x] Frontend: Chapters panel (parts, chapters, appendices; open, move, make
      appendix/chapter, remove, add chapter/part/appendix; list changes save the
      project file at once), chapter switching (saves the open chapter), numbering
      from the index (`bookNumbering`: the open chapter starts where the ones before
      end, CSS counters too, appendix chapters in letters), references to other
      chapters (picker, labels, Ctrl-click opens the chapter at the target), ids
      unique across chapters, citations in book order, chapter / book word count,
      PDF ↔ editor jumps and error jumps across chapters; Open takes `.loomproj` and
      chapter files (opens the project there); Split into project / Join into single
      document; PDF and LaTeX export of the whole book
- [ ] Frontend: search across chapters, the Outline with the other chapters
- [x] Compile switch (this chapter / whole book)
- [ ] CLI: `render`, `export` and `check` accept a `.loomproj`
- [x] New project (File menu and start window: a folder, the current template, a first
      chapter titled in the document's language)
- [x] An example project (start window and File menu): the Arabic thesis example in
      two parts when the interface is Arabic, else the senior design report; split
      into chapter files in a temporary folder when opened (`sample_project`), so it
      can't go out of date with the single-file examples

#### Decided

- Parts ("الباب الأول") are a level above chapters, in the project's chapter list and
  the Chapters panel, with their own style in the template.
- Front matter written as text (dedication, acknowledgments) stays as template
  metadata fields, not chapter files.
- When a one-chapter compile could show outdated numbers, the app compiles the whole
  book automatically.

### Phase 6: Polish and distribution (ongoing)
- [x] Installed TeX as an alternative engine (pdfLaTeX, XeLaTeX, LuaLaTeX via latexmk,
      found on PATH or in TeX Live/MacTeX folders; 3-minute limit per run);
      generated classes adapt (classic fonts under pdfLaTeX, kit stand-ins only for
      XeTeX/LuaTeX, plain tables for REVTeX)
- [x] Installers: Ubuntu .deb (CI on 22.04), snap (from the .deb), Windows .exe/.msi
      (GitHub Actions); app icon; no console windows for TeX runs on Windows
- [x] Bundled TeX package cache (the built-in templates and beamer themes compile
      offline from the first start)
- [ ] Auto-update; macOS build
- [x] Crash recovery: unsaved changes copied to the app data folder (with figures)
      3 s after each edit; offered back on the next start; closing asks first
- [x] Importing a kit into a built-in template asks for a name (My templates), then saves it
      with the kit's files (no more New copy → Save as → import)
- [x] UI redesign: title bar with File menu and Write/Design switch, icon toolbar
      (Lucide) with paragraph style dropdown and contextual tools, side rail with
      Outline / Document info / References / Checks, preview header with zoom,
      status bar (compile time, pages, words, template, page zoom); light/dark and
      compact themes
- [x] Settings window (settings.json in the config folder) and recent files
- [x] Template library ("My templates"): import a publisher kit (.zip → journal
      template around its class, sample's options and BibTeX style, standard title
      block), a template package or a template .json; export as a package; gallery
- [ ] Shared online template gallery
- [x] SyncTeX: double-click the PDF to go to that block; Ctrl+J shows the cursor's block
      in the PDF (codegen records each top-level block's .tex line; Tectonic --synctex,
      -synctex=1 for installed TeX)
- [x] Compile errors as a list with plain-language hints and the block they come from;
      clicking one goes there

### Later
Import of a .tex subset, syntax highlighting
in raw-LaTeX blocks (CodeMirror), collaboration (Yjs), instant preview with Typst.

## Risks

| Risk | Mitigation |
|---|---|
| Editor approximation and PDF look different | The template drives both the CSS and the .cls; the PDF preview is always visible |
| A visual option LaTeX can't express reliably | Map only to well-understood packages; raw-preamble escape hatch |
| Tectonic speed / first-run downloads | Debounced background compile; bundled package cache |
| WebKitGTK quirks | Cross-platform testing from Phase 0 |
| Scope creep | Senior design report end-to-end is the single target |
