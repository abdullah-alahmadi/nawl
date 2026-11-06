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

**TeX engine** (Settings → Preview): Texloom compiles with the bundled Tectonic by
default, so nothing else needs installing. If TeX Live, MiKTeX or MacTeX is installed,
you can use its **pdfLaTeX**, **XeLaTeX** or **LuaLaTeX** instead (through `latexmk`
when available): it works offline, reuses your installed packages, and runs publisher
kits made for pdfLaTeX as they are. With pdfLaTeX, templates use the TeX Gyre fonts as
classic font packages; system fonts need XeLaTeX or LuaLaTeX. The CLI takes
`--engine pdflatex|xelatex|lualatex|tectonic`, and Export LaTeX writes files for the
chosen engine.

## Writing a document

Texloom starts with a start window: pick a template for a new document (reports,
journals and conferences, or your own under **My templates**), open a document or a
recent file, or open the example report. Close it (Esc) to start with an empty page.

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
as things move; **Citation** inserts a reference from the **References** panel.
There, **New reference…** opens a form (journal article, conference paper, book,
chapter, thesis, report or website) with a row per author; the citation key is made
from the first author, year and title. Click a reference to edit or delete it. You
can also import or paste BibTeX (e.g. exported from Zotero). **Appendices** starts
the appendices; chapters after it are lettered. With the cursor in a heading,
**Numbered** on the right turns its number off (Acknowledgment, Preface); it stays
in the table of contents. As in LyX, you can't stack empty lines or spaces: LaTeX ignores
them, so Enter on an empty line does nothing, a space next to a space (or at the start
of a line) is ignored, and pasted text has its runs of spaces shrunk to one. Code keeps
every space.

The rail on the left opens the **Outline**, **Document info** (title, authors,
abstract… as the template defines them), **References**, and **Things to check**
(missing required fields, broken references, citations without an entry). The
status bar shows the compile state, page and word counts, and the page zoom.

**Settings** (gear icon, Ctrl+,): page zoom, the page color (warm paper by default; presets, dark pages, or any color), spell checking, crash recovery,
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
| Presentation (Beamer) | `beamer` (theme, colours and 16:9 / 4:3 under Design) |

Journal templates use the publisher's official class, so layout, fonts and captions
are exactly the publisher's; Texloom writes the title block, authors (edited in
**Document info**: affiliations, e-mail, ORCID, membership, corresponding author,
biography and photo), abstract, keywords and the bibliography in each publisher's
format. Start one with **File → New from template…**.

## Presentations

Choose **Presentation (Beamer)** in the template gallery (or **File → Open the example
presentation**). A presentation is a list of slides with section headings between them:

- Slides run down the page like any document, each at its real shape (16:9 or 4:3),
  with section headings between them. Content that runs past a slide's bottom edge sits
  on a red tint, because it won't fit on the slide in the PDF.
- The **Outline** panel lists the sections and numbered slides; click one to jump there.
- The toolbar adds **Slide**, **two columns**, a **box** (beamer's block; switch it to
  alert or example in its corner) and a **pause** (what follows appears on the next
  click). Lists, equations, figures, tables, citations and raw LaTeX work on slides too.
- The title slide comes from **Document info**: title, subtitle, short title (for the
  footer), presenters, institution, date and a **logo** (copied next to the document
  like figures). References go on slides of their own at the end.
- **Design template → Slides** decides where the logo goes (every slide, title slide
  only, off) and what the **footer** shows: the theme's own, none, or a custom left /
  centre / right line with `{author}`, `{title}`, `{date}`, `{institute}`, `{section}`,
  `{page}` and `{pages}`. The editor shows the footer and logo on each slide.
- **Theme** in the toolbar opens the theme picker: every beamer theme and colour
  theme as a preview rendered by the real theme, including the themes your TeX
  installation has (about 70 more on a full TeX Live; ones that need an institution's
  logo files are left out), and a **main colour** of your own for title bars, bullets
  and blocks. Previews are cached, so they're instant the second time.
- **Design template → Slides** has the same settings plus font theme, slide shape
  (16:9, 16:10, 4:3), text size, outline slides and navigation symbols. The editor
  approximates the theme's colours; the PDF preview is exact.

A presentation opened with an article template still compiles: slide titles become
bold lines, columns become side-by-side boxes, and pauses are dropped.

## Templates for any journal

**Import template…** (in **File → New from template…** and in the title bar's
template menu) adds a template to **My templates**. It accepts:

- **A publisher's LaTeX kit** (the .zip from the journal's author guidelines, e.g.
  Springer Nature, MDPI, Taylor & Francis): Texloom makes a journal
  template around the kit's class, with the class options and BibTeX style its
  sample paper uses. Classes Texloom knows get their proven title block (IEEE Access,
  IEEEtran, elsarticle, llncs, acmart, REVTeX); others get a standard title block.
  If the publisher wants a special
  title block, adjust it under **Design template → Publisher class** (a *Custom
  recipe*: LaTeX with placeholders such as `{title}` and
  `{#authors| \and }{name}{/authors}`; examples and a placeholder list are there).
- **A template package** (.zip) that someone exported with **Export template…**.
- **A Texloom template file** (.json) with its `assets/` folder.

My templates live in Texloom's data folder; remove one from the gallery (trash icon).
**Export template…** saves the current template and its files as a .zip to share.

### Journals from your TeX installation

With TeX Live installed, the gallery also lists **From your TeX installation**: every
journal or conference class TeX Live's package database files under the TeX
Catalogue's *journalpub* topic (MNRAS, Nature, ACS, AAS, Oxford University Press,
Elsevier CAS, JMLR, Quantum, APA, ASCE, AIAA, LNI and more, about 40 on a full
TeX Live). Nothing is copied: the template uses the class as installed. Well-known
classes get a tuned title block; the rest the standard one. Classes that need
special fonts or title commands Texloom can't write yet are left out. The gallery's
search box filters all templates by name, description or class.

To check the list against your installation (prints which classes compile with the
sample article):

```bash
cargo test -p texloom installed_journal -- --ignored --nocapture
```

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
