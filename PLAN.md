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
  Later (zip): adds `assets/`, `refs.bib`/`refs.yaml` and an embedded copy of the template.
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
- [ ] .loom as a single zip (document + images); new/blank document command
- [ ] Table cells spanning rows; image cropping

### Phase 3: Template designer v1, styles (~3–4 weeks)
- [x] Template mode with panels: info, page, fonts (built-in + system), structure &
      numbering, headings (with instant on-screen sample), headers/footers (placeholder
      chips), fields, front matter
- [x] Live preview: debounced validation → PDF of the current document; errors shown
      inline and per section while the last valid version stays in the preview
- [x] Save / Save as (copies image assets) / New copy; Ctrl+S saves the template in Template mode
- [ ] Instant CSS preview of the editor page from the template (pairs with Phase 2's
      template-driven editor styling)
- [ ] Undo/redo inside the designer; caption and TOC styling once figures exist

### Phase 4: Title-page designer (~2–3 weeks)
- [x] Page-sized canvas in mm: drag to move, right-edge handle to resize, snap to the
      page centre, arrow-key nudging (Shift ×10, Alt 0.1 mm), anchor picker, Centre button
- [x] Text with `{field}` placeholders (shown with the open document's values), images
      (imported into the template's `assets/`), lines → TikZ overlay
- [x] Canvas geometry matches the PDF (checked against the PDF's word positions: within
      ~0.5 mm; line spacing follows the template's setspace factor)
- [ ] Extra fixed pages built the same way (approval/signature, declaration)
- [ ] Snapping to other items / margins; multi-select; undo

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
- [ ] Posters (tikzposter / beamerposter): a big canvas of boxes in columns
- [ ] Letters (letter / scrlttr2): sender, recipient, opening, closing
- [ ] CVs (moderncv): sections of dated entries
- [ ] Books and theses: parts, front/back matter on top of the report template
- [ ] Presentations next: slide layouts, speaker notes, overlays beyond \pause,
      reordering slides by dragging

### Phase 6: Polish and distribution (ongoing)
- [x] Installed TeX as an alternative engine (pdfLaTeX, XeLaTeX, LuaLaTeX via latexmk,
      found on PATH or in TeX Live/MacTeX folders; 3-minute limit per run);
      generated classes adapt (classic fonts under pdfLaTeX, kit stand-ins only for
      XeTeX/LuaTeX, plain tables for REVTeX)
- [ ] Bundled TeX package cache so it works offline; installers; auto-update
- [x] Crash recovery: unsaved changes copied to the app data folder (with figures)
      3 s after each edit; offered back on the next start; closing asks first
- [x] Importing a kit into a built-in template asks where to save it, then saves it
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

### Later
SyncTeX (jump between PDF and source), import of a .tex subset, syntax highlighting
in raw-LaTeX blocks (CodeMirror), collaboration (Yjs), instant preview with Typst.

## Risks

| Risk | Mitigation |
|---|---|
| Editor approximation and PDF look different | The template drives both the CSS and the .cls; the PDF preview is always visible |
| A visual option LaTeX can't express reliably | Map only to well-understood packages; raw-preamble escape hatch |
| Tectonic speed / first-run downloads | Debounced background compile; bundled package cache |
| WebKitGTK quirks | Cross-platform testing from Phase 0 |
| Scope creep | Senior design report end-to-end is the single target |
