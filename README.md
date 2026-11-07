<p align="center"><img src="assets/texloom-icon.png" width="128" height="128" alt="Texloom icon"></p>

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
type the caption below it; **Crop…** cuts off its edges by dragging), **tables**
(caption above; row and column buttons appear on the right while the cursor is in the
table, and **Merge** joins the cells you drag across, spanning rows and columns) and
**raw LaTeX blocks** (written into the PDF exactly as typed, for TikZ drawings and the
like). Figure images are stored in a `figures/` folder next to the `.loom` file;
**File → Save as single file…** puts them inside the `.loom` instead, to email or
upload one file (saving it again keeps it one file).

**Ctrl+F** finds text in the document (headings, paragraphs, captions, tables) and
**Ctrl+H** replaces it, one match at a time or all at once (one undo).

The preview and the editor are linked: **double-click the PDF** to put the cursor in
the paragraph, heading or figure you clicked, and **Ctrl+J** (or the target button
above the preview) scrolls the PDF to the cursor's place and marks it (SyncTeX). When
LaTeX reports errors, the list says what each one usually means and where it is
("in Heading “Results”"); click one to go there.

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

**File → New from template…** (Ctrl+Shift+N) shows every template: Texloom's own, your
**My templates**, and the classes of your TeX installation. Category chips (Reports,
Presentations, Journals, Theses, Books, Articles, My templates) narrow the list, the
search box looks through all of them, and the view switches between cards and a compact
list. The templates you started documents with last appear first under **Recently
used**; the window remembers the category and view. The template button in the title
bar switches the open document to another template. Texloom's own templates:

| Template | Built on |
|---|---|
| Senior Design Report | Texloom's own layout (fully editable in the designer) |
| Article | Texloom's own layout: one column, A4, title block with abstract and keywords on page 1 |
| Two-column Article | Texloom's own layout: journal style, title and abstract across both columns, running head |
| Short Paper | Texloom's own layout: compact one-column paper on US Letter for workshops and letters |
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

## Arabic documents

Texloom typesets Arabic and mixed Arabic/English documents: journal articles, theses,
books and presentations, right to left, with Arabic fonts, headings and labels.

- **Templates:** the gallery's **العربية** (Arabic) category has an Arabic journal
  article (Arabic title, abstract and keywords, then the English ones), a thesis (title
  page, approval page, Arabic and English abstracts, chapters), a book (two-sided,
  170×240 mm) and a presentation. **New blank template…** in the template menu (title
  bar) can start a blank Arabic template, and the
  designer's **Language** section turns any template Arabic: main language, second
  language, and Arabic-Indic (١٢٣) or Western (123) digits for page, chapter, figure and
  equation numbers.
- **Writing:** type Arabic and English freely; Texloom finds the English words and
  marks them for LaTeX itself (no `\textenglish` by hand). A whole paragraph in the other
  language: **Ctrl+Shift+X** or the direction button in the toolbar. Numbers with a
  decimal point stay left to right.
- **Fonts:** Amiri (part of TeX Live and Tectonic) by default, or any installed font in
  the designer's **Arabic font**, which lists Windows fonts such as Traditional Arabic and
  Simplified Arabic too. English text uses the template's Latin fonts.
- **References:** Arabic entries are formatted in Arabic (وآخرون، د.ت.) and English
  ones in English, in the same list, for IEEE, numeric and author-year styles.
- **Engines:** Arabic needs XeLaTeX or LuaLaTeX; the built-in Tectonic is XeLaTeX.
  pdfLaTeX can't typeset Arabic, so with pdfLaTeX chosen in Settings, Arabic documents
  still compile with Tectonic (or an installed XeLaTeX), and Export LaTeX writes them
  for XeLaTeX. Arabic slides need LuaLaTeX (beamer-rl), from TeX Live or MiKTeX.
- **Word:** **File → Export Word…** writes a `.docx` for anyone who still asks for Word:
  right-to-left paragraphs and tables, Arabic and English fonts, headings Word's
  navigation pane and table of contents understand, editable equations, figures (with
  their crop), tables with merged cells, captions, the reference list and page numbers.
  Texloom doesn't open Word files; the `.loom` stays the source.
- **Interface:** **Settings → Appearance → Language** switches the interface to Arabic
  (right to left). It follows the system language by default. The editor page keeps the
  document's own direction.

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

### Templates from your TeX installation

With TeX Live installed, the gallery also lists **From your TeX installation**: the
document classes in TeX Live's package database, grouped by kind using the TeX
Catalogue's topics: **journals and conferences** (MNRAS, Nature, ACS, AAS, Oxford
University Press, Elsevier CAS, JMLR, Quantum…), **theses and dissertations**,
**books**, and **articles and reports** (KOMA-Script, memoir and the like). Book and
thesis templates start with chapters. Nothing is copied: the template uses the class
as installed. Well-known classes get a tuned title block; the rest the standard one.
Classes that don't compile with Texloom's standard title block (they need their own
commands, fonts or files) are left out. The template menu's **From your TeX
installation** entry opens the same list, with the presentation template and your
installed beamer themes.

Letter, CV, poster, exam and non-beamer presentation classes are counted but not
offered yet: Texloom can't write those kinds of documents. With MiKTeX (no TeX Live
package database) only the classes Texloom knows by name are found. The gallery's
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
them. Use **New copy** to start from the current template, then **Save as…**. To
design one from scratch, use **New blank…** (or **New blank template…** in the title
bar's template menu): name it, pick report or thesis, article or presentation, and the
paper size. You get a plain template (title, authors and date; simple headings; a
basic title page) saved in **My templates** and opened in the designer.
The **Title page** panel is a drag-and-drop canvas: add text, images and lines, drag
them into place, and edit text, style, size and position on the right. Items snap to
the page's centre and margins and to each other's edges (hold Alt for none). Select
several with Shift-click or by dragging a frame around them, then move them together
or line them up (**Align**: left edges, centres, tops…; **Spread** for equal gaps).
**+ Fixed page** adds a page laid out the same way, such as an approval or signature
page or a declaration; it goes into the front matter after the title page (move it
under **Front matter**) and can be listed in the table of contents. Images are copied
into the template's `assets/` folder, so save a new template once (Save as…) before
adding them.

For articles, set the Title page panel's **Layout** to **A block at the top of page 1**:
the title, authors and affiliation sit above the text instead of on a page of their
own, and **Text starts at** says where the text begins (the canvas marks it). A title
or author list longer than designed pushes the text down rather than running into it.
Under **Front matter**, **Under the title on the first page** puts the abstract (or any
paragraphs field) right below the block. **Page → Columns** sets one or two columns;
with two, the title block and abstract span both. The editor keeps one column, and the
PDF and Word export have two.

While designing, the preview's **Page** view shows the document's page with the
template as you change it, without waiting for a compile; **PDF** is the exact
version.

**Save as…** asks for a name and saves the template into **My templates** (the app's
data folder, one folder per template, images and kit files included). It then appears
under My templates in **File → New from template…** and the title bar's template menu,
ready for new documents; **Save template** (Ctrl+S) keeps saving it in place. To share
it, use **Export template…** (a `.zip` others can import).

Template files are plain JSON (`crates/core/src/template.rs` documents every option)
with images in an `assets/` folder next to them. To keep one elsewhere, for example to
edit it by hand, choose **Save to file…** in the Save as dialog. Open such a file with
**Open template file…** in the title bar's template menu (or `TEXLOOM_TEMPLATE=path npm run tauri dev`)
and every save in your editor reloads it.

Saved documents remember their template (path plus an embedded copy), so they reopen
with it.

## Command line

```bash
cargo run -p texloom-cli -- render examples/senior-design-report.loom -t templates/senior-design/template.json -o report.pdf
cargo run -p texloom-cli -- export examples/senior-design-report.loom -t templates/senior-design/template.json --out-dir build/
cargo run -p texloom-cli -- check templates/senior-design/template.json --print-cls
```

## Packages

Installers are built by GitHub Actions (`.github/workflows/packages.yml`): the Ubuntu
`.deb` (built on Ubuntu 22.04, so it installs on 22.04 and newer), a snap made from that
`.deb` (`snap/snapcraft.yaml`), and the Windows installers (`.exe` and `.msi`). Run the
**Packages** workflow from the Actions tab, or push a version tag to also get a draft
release with the packages attached:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Locally, on Linux:

```bash
scripts/fetch-tectonic.sh
npm run tauri build -- --bundles deb
sudo apt install ./target/release/bundle/deb/Texloom_0.1.0_amd64.deb
```

The snap is strictly confined: it compiles with the bundled Tectonic, and can't see a
TeX Live installed on the system (use the `.deb` for that). The app icon's source is
`assets/texloom-icon.svg` (`npx tauri icon assets/texloom-icon.png` regenerates the sizes).

## Tests

```bash
cargo test --workspace            # also regenerates src/bindings/*.ts
INSTA_UPDATE=always cargo test -p texloom-codegen   # accept changed .cls/.tex snapshots (review the diff!)
TEXLOOM_TECTONIC=$PWD/src-tauri/binaries/tectonic-x86_64-unknown-linux-gnu cargo test --workspace -- --ignored   # real compiles
```
