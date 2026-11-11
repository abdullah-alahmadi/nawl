<p align="center"><img src="assets/nawl-icon.png" width="128" height="128" alt="Nawl icon"></p>

<h1 align="center">Nawl · نول</h1>

<p align="center">
  A visual editor for LaTeX documents, with templates you design without code.<br>
  Built Arabic first.
</p>

<p align="center">
  <b>English</b> · <a href="README.ar.md">العربية</a> ·
  <a href="docs/help.md">User guide</a> · <a href="docs/help.ar.md">دليل الاستخدام</a>
</p>

![Nawl: the editor on the left, the PDF on the right](docs/images/en-editor.png)

Nawl (نَوْل, Arabic for *loom*) is in the spirit of LyX: you write on a page that looks
like the result, and Nawl writes the LaTeX and shows the PDF beside it. Templates (the page, the fonts,
the headings, the title page) are designed visually instead of in code.

> **Early preview (version 0.1).** Nawl is usable for real documents, but it is young:
> expect rough edges, and keep copies of work that matters. File formats may still change
> between versions. The installers are not code-signed, so Windows shows a SmartScreen
> warning on first run. It is developed and tested on Windows and Ubuntu; macOS is
> untested. Bug reports are welcome in the
> [issues](https://github.com/abdullah-alahmadi/nawl/issues).

## Contents

- [What it does](#what-it-does)
- [A tour in pictures](#a-tour-in-pictures)
- [Install](#install)
- [Documentation](#documentation)
- [Building from source](#building-from-source)
- [Command line](#command-line)
- [Packages](#packages)
- [Tests](#tests)
- [License](#license)

## What it does

- **Writes like a word processor, typesets with LaTeX.** Headings, lists, figures,
  tables, equations, cross-references and citations, with a live PDF preview linked to
  the editor in both directions.
- **Templates without code.** A visual designer for the page, fonts, headings, title
  page and front matter. Publisher classes (IEEE, Elsevier, ACM, Springer, APS) and the
  classes of your own TeX installation work too.
- **Arabic first.** Right-to-left articles, theses, books, administrative orders and
  slides; a library of 31 free Arabic fonts; Arabic references, dates and digits; an
  Arabic interface.
- **Arabic mathematics.** Formulas written right to left, with mirrored integrals, sums
  and roots and Arabic function names.
- **Projects.** A thesis or book as one project with a file per chapter.
- **Presentations** with beamer, and export to PDF, LaTeX and Word.
- **Nothing else to install.** The TeX engine (Tectonic) and the packages the built-in
  templates need come with the app, so documents compile offline.

## A tour in pictures

### Start from a template

Reports, articles, journals, theses, books and presentations, in English and Arabic.
Your own templates and the classes of an installed TeX Live appear here too.

![The start window with the template gallery](docs/images/en-start.png)

### Write, and see the PDF

The page in the editor is styled by the template. Double-click the PDF to jump to that
place in the editor; Ctrl+J goes the other way.

![An equation being edited, with the PDF beside it](docs/images/en-equation.png)

### Arabic documents

Arabic and English mix freely. Nawl finds the English words itself and keeps each
language in its own direction and font.

![An Arabic article in the Arabic interface](docs/images/ar-editor.png)

### Arabic mathematics

Formulas can run right to left, with Arabic letters, mirrored symbols and Arabic
function names. A helper converts letters and inserts Arabic symbols.

![Arabic mathematics with the formula helper](docs/images/ar-math-helper.png)

### Design the template

Every part of the layout has a panel, and the title page is a drag-and-drop canvas.

![The template designer's title page canvas](docs/images/en-designer-titlepage.png)

A gallery of Arabic fonts, each shown in its own face:

![The Arabic font gallery](docs/images/ar-designer-fonts.png)

### A thesis or book as chapters

A project keeps one file per chapter. Numbers, cross-references and citations work
across chapters.

![A project with its Chapters panel](docs/images/ar-project.png)

### Presentations

Slides with beamer, at their real shape, with every beamer theme to choose from.

![A presentation](docs/images/en-slides.png)

### Official documents

An Arabic administrative order with a letterhead, both calendars and a signature.

![An Arabic administrative order](docs/images/ar-order.png)

## Install

Installers are attached to the
[releases](https://github.com/abdullah-alahmadi/nawl/releases): `.exe` and `.msi` for
Windows, `.deb` and a snap for Ubuntu. If no release is listed yet, build from source as
described below.

On Windows the installer is not code-signed: choose **More info → Run anyway** when
SmartScreen warns.

## Documentation

| | English | العربية |
|---|---|---|
| User guide, with pictures | [docs/help.md](docs/help.md) | [docs/help.ar.md](docs/help.ar.md) |
| This page | [README.md](README.md) | [README.ar.md](README.ar.md) |
| Detailed notes on each feature | [docs/reference.md](docs/reference.md) | |
| Roadmap and design notes | [PLAN.md](PLAN.md) | |

In the app, **File → Help** (F1) opens the user guide in the interface's language.

## Building from source

On Ubuntu:

```bash
sudo apt install -y build-essential curl wget file libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev nodejs npm rustup
rustup default stable
npm install                 # also copies MathLive fonts into public/
scripts/fetch-tectonic.sh   # downloads Tectonic into src-tauri/binaries/
pip install fonttools && node scripts/fetch-fonts.mjs   # the Arabic font library (optional)
npm run tauri dev
```

On Windows, install Rust, Node.js and the
[Tauri prerequisites](https://tauri.app/start/prerequisites/), then run the same
commands from Git Bash with the Windows engine:
`scripts/fetch-tectonic.sh x86_64-pc-windows-msvc`.

The first compile takes a minute or two while Tectonic downloads its TeX package bundle;
after that it's cached. The installers instead ship the packages the built-in templates
need (`node scripts/build-tex-cache.mjs`), so they compile without internet from the
first start; other packages still download when needed.

**TeX engine** (Settings → Preview): Nawl compiles with the bundled Tectonic by
default. If TeX Live, MiKTeX or MacTeX is installed, you can use its **pdfLaTeX**,
**XeLaTeX** or **LuaLaTeX** instead (through `latexmk` when available): it works
offline, reuses your installed packages, and runs publisher kits made for pdfLaTeX as
they are. The CLI takes `--engine pdflatex|xelatex|lualatex|tectonic`, and Export LaTeX
writes files for the chosen engine.

The code is laid out as:

| Folder | What it holds |
|---|---|
| `crates/core` | The template and document model, and validation |
| `crates/codegen` | Template → `.cls`, document → `.tex` |
| `crates/compile` | Runs Tectonic or an installed TeX |
| `crates/docx` | Word export |
| `crates/cli` | The command-line tool |
| `src-tauri` | The app's backend |
| `src` | The frontend (React and TipTap) |
| `templates`, `examples` | Built-in templates and sample documents |

## Command line

```bash
cargo run -p nawl-cli -- render examples/senior-design-report.loom -t templates/senior-design/template.json -o report.pdf
cargo run -p nawl-cli -- export examples/senior-design-report.loom -t templates/senior-design/template.json --out-dir build/
cargo run -p nawl-cli -- check templates/senior-design/template.json --print-cls
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
node scripts/fetch-fonts.mjs         # Arabic font library (needs: pip install fonttools)
node scripts/build-tex-cache.mjs     # TeX packages for compiling offline
npm run tauri build -- --bundles deb
sudo apt install ./target/release/bundle/deb/Nawl_0.1.0_amd64.deb
```

On Windows, `npm run tauri build -- --bundles nsis,msi`.

The snap is strictly confined: it compiles with the bundled Tectonic, and can't see a
TeX Live installed on the system (use the `.deb` for that). The app icon's source is
`assets/nawl-icon.svg` (`npx tauri icon assets/nawl-icon.png` regenerates the sizes).

## Tests

Every push to `main` runs the tests on GitHub (`.github/workflows/tests.yml`).

```bash
cargo test --workspace            # also regenerates src/bindings/*.ts
npx tsc --noEmit -p .             # frontend type check
INSTA_UPDATE=always cargo test -p nawl-codegen   # accept changed .cls/.tex snapshots (review the diff!)
NAWL_TECTONIC=$PWD/src-tauri/binaries/tectonic-x86_64-unknown-linux-gnu cargo test --workspace -- --ignored   # real compiles
cargo test -p nawl installed_journal -- --ignored --nocapture   # which installed TeX classes compile
```

## License

Nawl is licensed under the [Apache License, Version 2.0](LICENSE).
Copyright 2026 Abdullah Al-Ahmadi.

The installers also carry software and fonts by others under their own licenses:
Tectonic and MathLive (MIT), TeX packages (mostly the LaTeX Project Public License),
and the Arabic font library (see [NOTICE](NOTICE)).
