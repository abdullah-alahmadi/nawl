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

## Making a template

Switch to **Template** mode (top left). The designer has panels for template info,
page, fonts, structure, headings, headers & footers, fields and front matter; every
change updates the PDF preview, and problems appear next to the control that caused
them. Use **New copy** to start from the current template, then **Save as…**.
(The title page is still edited in the JSON file; its visual canvas is Phase 4.)

Template files are plain JSON (`crates/core/src/template.rs` documents every option)
with images in an `assets/` folder next to them. You can also edit one by hand: open it
with **Template ▾ → Open template file…** (or `TEXLOOM_TEMPLATE=path npm run tauri dev`)
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
