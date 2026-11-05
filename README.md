# Texloom

Texloom is a LyX-like visual LaTeX editor built with Tauri, whose templates are designed visually
instead of in code. See [PLAN.md](PLAN.md) for the roadmap.

## Setup (Linux)

```bash
sudo apt install -y build-essential curl wget file libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev nodejs npm rustup
rustup default stable
npm install                 # also copies MathLive fonts into public/
scripts/fetch-tectonic.sh   # downloads Tectonic into src-tauri/binaries/
npm run tauri dev
```

The first compile takes a minute or two while Tectonic downloads its TeX package bundle;
after that it's cached in `~/.cache/Tectonic`.

## Writing a document

The window has a **File** menu (New, Open, recent files, Save, Export) and a
**Write / Design template** switch at the top. The editor page is styled by the
template, so it looks like the PDF next to it.

The toolbar has the **paragraph style** (Body text, Chapter, Section… as in LyX),
bold/italic/code, lists, and inserts: **inline math** (type `$x^2$` in a sentence,
or Ctrl+M), **equations** (`$$` + space; hover one to make it unnumbered or
multi-line, with **+ Line** for aligned derivations), **figures** (pick an image;
type the caption below it), **tables** (caption above; row and column buttons
appear on the right while the cursor is in the table) and **raw LaTeX blocks**
(written into the PDF exactly as typed, for TikZ drawings and the like). Figure
images are stored in a `figures/` folder next to the `.loom` file.

**Cross-reference** inserts "Figure 3.1", "Chapter 2" or "(3.2)" and stays correct
as things move; **Citation** inserts a reference from the **References** panel,
where you import or paste BibTeX (e.g. exported from Zotero). **Appendices** starts
the appendices; chapters after it are lettered. With the cursor in a heading,
**Numbered** on the right turns its number off (Acknowledgment, Preface); it stays
in the table of contents. As in LyX, you can't stack empty lines: LaTeX ignores
them, so Enter on an empty line does nothing.

The rail on the left opens the **Outline**, **Document info** (title, authors,
abstract… as the template defines them), **References**, and **Things to check**
(missing required fields, broken references, citations without an entry). The
status bar shows the compile state, page and word counts, and the page zoom.

**Settings** (gear icon, Ctrl+,): page zoom, spell checking, crash recovery,
compiling while typing or only on Ctrl+Enter, light/dark theme, compact density,
the default folder, the number of recent files, and a list of keyboard shortcuts.

Unsaved changes are copied to Texloom's data folder a few seconds after each edit. If
Texloom closes without saving (a crash, a power cut), it offers to restore them on the
next start.

## Built-in templates

**File → New from template…** (Ctrl+Shift+N) shows the templates that ship with Texloom; the template button in the title bar switches the open document to another one:

| Template | Built on |
|---|---|
| Senior Design Report | Texloom's own layout (fully editable in the designer) |
| IEEE Journal / Transactions | `IEEEtran` (journal) |
| IEEE Conference | `IEEEtran` (conference) |
| Elsevier Journal Article | `elsarticle` (options `1p`/`3p`/`5p` for journal layouts) |
| ACM Conference Paper | `acmart` (`sigconf`; use `acmsmall` for ACM journals) |
| Springer LNCS | `llncs` |
| APS Physical Review | `revtex4-2` (`pra`; use `prl` for Letters) |
| IEEE Access | IEEE's own kit (`ieeeaccess.cls`, fonts, logos): download once, then import the .zip |

Journal templates use the publisher's official class, so layout, fonts and captions
are exactly the publisher's; Texloom writes the title block, authors (edited in
**Document info**: affiliations, e-mail, ORCID, membership, corresponding author,
biography and photo), abstract, keywords and the bibliography in each publisher's
format. Start one with **File → New from template…**.

**IEEE Access**: choose it, open the **Template** tab, and under **Publisher class**
follow the card: download IEEE's kit and **Import kit (.zip)**. Texloom asks where
to save the template (the kit is kept next to it) and saves it with the kit.
Texloom adapts the kit's pdfLaTeX-only parts (font maps, PANTONE spot colour) for its
XeTeX-based engine; the result matches IEEE's own sample PDF.

Other journals whose class isn't in the TeX distribution (Springer Nature, MDPI,
Taylor & Francis…): start from the closest built-in template, choose **New copy**,
import the publisher's kit, and pick a title block style — or choose
**Custom recipe** and write the title block once as LaTeX with placeholders such as
`{title}` and `{#authors| \and }{name}{/authors}` (examples and a placeholder list are
in the designer).

## Making a template

Switch to **Template** mode (top left). The designer has panels for template info,
page, fonts, structure, headings, headers & footers, fields and front matter; every
change updates the PDF preview, and problems appear next to the control that caused
them. Use **New copy** to start from the current template, then **Save as…**.
The **Title page** panel is a drag-and-drop canvas: add text, images and lines, drag
them into place (they snap to the page centre), and edit text, style, size and
position on the right. Images are copied into the template's `assets/` folder, so save
a new template once (Save as…) before adding them.

Template files are plain JSON (`crates/core/src/template.rs` documents every option)
with images in an `assets/` folder next to them. You can also edit one by hand: open it
with **Open template file…** in the title bar's template menu (or `TEXLOOM_TEMPLATE=path npm run tauri dev`)
and every save in your editor reloads it.

Saved documents remember their template (path plus an embedded copy), so they reopen
with it.

## Command line

```bash
cargo run -p texloom-cli -- render examples/senior-design-report.loom -t templates/senior-design/template.json -o report.pdf
cargo run -p texloom-cli -- export examples/senior-design-report.loom -t templates/senior-design/template.json --out-dir build/
cargo run -p texloom-cli -- check templates/senior-design/template.json --print-cls
```

## Tests

```bash
cargo test --workspace            # also regenerates src/bindings/*.ts
INSTA_UPDATE=always cargo test -p texloom-codegen   # accept changed .cls/.tex snapshots (review the diff!)
TEXLOOM_TECTONIC=$PWD/src-tauri/binaries/tectonic-x86_64-unknown-linux-gnu cargo test --workspace -- --ignored   # real compiles
```
