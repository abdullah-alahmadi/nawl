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
| Rust ↔ TS types | tauri-specta (or ts-rs) |
| TeX engine | Tectonic, bundled as a sidecar binary (may move to the crate later) |
| Bibliography | hayagriva (CSL) → `thebibliography` (no biber dependency) |
| Codegen | minijinja |
| Tests | insta (snapshots of generated LaTeX), Playwright (UI) |

## Repository layout

```
crates/
  core/      Document + Template models (serde), validation. No UI, no TeX.
  codegen/   Template → .cls, Document → .tex (minijinja templates)
  compile/   Tectonic runner, log parsing → friendly errors
  biblio/    hayagriva wrapper
  cli/       `texloom render doc.json --template t.json -o out.pdf`
src-tauri/   Tauri app: commands that call the crates
src/         React frontend (Tauri's default layout keeps it at the repo root)
  editor/    TipTap schema + custom nodes (equation, figure, citation, ref)
  designer/  template designer panels + title-page canvas
  preview/   pdf.js pane
scripts/     fetch-tectonic.sh (sidecar binary), copy-mathlive-assets.mjs (postinstall)
```

## File formats

- **Document** (zip): `document.json` (ProseMirror JSON), `metadata.json`, `assets/`,
  `refs.bib`/`refs.yaml`, plus an embedded copy of its template.
- **Template** (zip): `template.json` + `assets/` (logos, fonts).
- **Export** (zip): `report.tex` + `<template>.cls` + assets + bibliography. Compiles
  anywhere, including Overleaf.

### Template model (sketch)

```rust
pub struct Template {
    pub meta: TemplateMeta,
    pub page: PageSetup,                 // paper, margins (mm), binding offset, spacing
    pub fonts: Fonts,                    // body / heading / mono
    pub styles: HashMap<ElementKind, ElementStyle>,
    pub numbering: Numbering,            // roman front matter, arabic body, chapter format
    pub header_footer: HeaderFooter,     // L/C/R slots per page type, with {fields}
    pub title_page: TitlePage,           // positioned items on a canvas (mm)
    pub fields: Vec<MetadataField>,      // title, team members, advisor, course…
    pub skeleton: Vec<SkeletonSection>,  // required/locked sections and order
    pub bibliography: BibStyle,          // CSL style id
    pub raw_preamble: Option<String>,    // escape hatch
}
```

Package mapping: geometry (page setup), fontspec (fonts), titlesec (headings), fancyhdr
(headers/footers), caption (captions), tocloft (table of contents), and TikZ
`remember picture, overlay` for the absolutely positioned title page.

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
- [ ] `core` structs + TS type export
- [ ] `codegen`: Template → .cls, Document → .tex; correct escaping of special characters
- [ ] `compile`: Tectonic runner + readable errors
- [ ] `cli` + snapshot tests; hand-written `senior-design.json` reference template

### Phase 2: Editor MVP (~3–4 weeks)
- [ ] TipTap schema matching `core`; toolbar, shortcuts, outline sidebar
- [ ] CSS generated from the template, applied to the editor
- [ ] Debounced background compile → preview pane
- [ ] Open/save document files; export PDF and the .tex zip

### Phase 3: Template designer v1, styles (~3–4 weeks)
- [ ] Panels: page setup, fonts, element styles, numbering, headers/footers
- [ ] Live sample preview (instant CSS + PDF a moment later)
- [ ] Save/load template files with bundled assets

### Phase 4: Title-page designer (~2–3 weeks)
- [ ] Page-sized canvas in mm: drag, resize, snap, align
- [ ] Text with `{field}` placeholders, images, rules → TikZ overlay
- [ ] Extra fixed pages built the same way (approval/signature, declaration)

### Phase 5: Structure and academic features (~3–4 weeks)
- [ ] Skeleton sections (required/locked), metadata form generated from `fields`
- [ ] Front matter, TOC/LoF/LoT, roman → arabic page numbering
- [ ] Labels + cross-references; citations from .bib with CSL styles; appendices
- [ ] **Milestone:** full senior design report from a template made entirely in the designer

### Phase 6: Polish and distribution (ongoing)
- [ ] Bundled TeX package cache so it works offline; installers; auto-update
- [ ] Template import/export and gallery; autosave and crash recovery

### Later
SyncTeX (jump between PDF and source), import of a .tex subset, raw-LaTeX blocks
(CodeMirror), collaboration (Yjs), instant preview with Typst.

## Risks

| Risk | Mitigation |
|---|---|
| Editor approximation and PDF look different | The template drives both the CSS and the .cls; the PDF preview is always visible |
| A visual option LaTeX can't express reliably | Map only to well-understood packages; raw-preamble escape hatch |
| Tectonic speed / first-run downloads | Debounced background compile; bundled package cache |
| WebKitGTK quirks | Cross-platform testing from Phase 0 |
| Scope creep | Senior design report end-to-end is the single target |
