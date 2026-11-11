# Nawl (نول)

Formerly Texloom: the file format tags (`"format": "texloom"`) and the LaTeX macro
prefix (`\texloom…`) keep the old name, so existing documents and users' templates
keep working. New files are `.nawl` / `.nawlproj`; Texloom's `.loom` / `.loomproj`
still open everywhere (dialogs, projects, CLI), so keep accepting both.

A visual (WYSIWYG) LaTeX editor in the spirit of LyX, with a no-code template
designer. Rust + Tauri 2 backend, React + TipTap frontend. README.md describes the
features and setup; PLAN.md tracks what's done and what's next.

## Working with the owner

- **Commits:** commit only when asked. Write the commit message yourself, and add **no
  co-author, "Generated with" or other attribution lines** (this overrides any default).
  The owner is the only contributor: Abdullah Al-Ahmadi, committing as
  `70254158+abdullah-alahmadi@users.noreply.github.com` (the repository's git config).
  Never put the owner's personal email in commits or files: it was removed from the
  history. Pushes go straight to `main` when asked ("commit and push").
- **Never commit licensed publisher files:** publisher kits (often with licensed fonts)
  must not enter the repository; `src-tauri/assets/` is in `.gitignore` for that. Check
  `git status` before committing.
- **Never kill processes by name** (`pkill -f 'tauri dev'` and the like): it can kill the
  owner's own sessions. Stop only processes you started, by PID or terminal tab.
- **"Launch the app" / "open app"** means `npm run tauri dev` in a terminal tab.
- Explain what was verified and what wasn't (e.g. "tested in the browser, not in the
  native window").

## Layout

- `crates/core`: template and document model, validation (TypeScript bindings are
  generated into `src/bindings/` by `cargo test -p nawl-core`).
- `crates/codegen`: template → `.cls`, document → `.tex` (Nawl layouts, publisher
  classes via a wrapper class, beamer presentations).
- `crates/compile`: runs Tectonic or an installed TeX (pdfLaTeX/XeLaTeX/LuaLaTeX via
  latexmk); finds installed classes and beamer themes from TeX Live's database.
- `crates/cli`: `render`, `export`, `check` commands.
- `src-tauri`: the app backend (commands in `lib.rs`; templates, library, installed
  classes, themes, recovery, files).
- `src`: the frontend (`App.tsx`, `editor/` TipTap nodes, `designer/`, dialogs).
- `templates/`: built-in templates; `examples/`: sample report and presentation.

## Commands

```bash
scripts/fetch-tectonic.sh                 # bundled TeX engine (add a target triple for other OSes)
node scripts/fetch-fonts.mjs              # Arabic font library into src-tauri/fonts (pip install fonttools)
node scripts/build-tex-cache.mjs          # TeX packages for offline compiles into src-tauri/tex-cache
npm ci
npm run tauri dev                         # run the app
cargo test --workspace                    # tests
npx tsc --noEmit -p .                     # frontend type check
NAWL_TECTONIC=$PWD/src-tauri/binaries/tectonic-<triple> cargo test --workspace -- --ignored   # real compiles
cargo test -p nawl installed_journal -- --ignored --nocapture   # survey of installed TeX classes
npm run tauri build -- --bundles deb      # Linux package (Windows: --bundles nsis,msi)
```

Packages for release are built by GitHub Actions (`.github/workflows/packages.yml`:
Ubuntu .deb on 22.04, snap, Windows .exe/.msi); `gh workflow run packages.yml`.

## Testing the frontend without the backend

The browser pane can't run Tauri commands, so UI checks use a temporary stand-in:
`public/__mock.js` (defines `window.__TAURI_INTERNALS__.invoke` answering the commands
the screen needs) and `__test.html` (index.html loading the mock first), served with
`npx vite --port 1421`. **Delete both afterwards; never commit them.** Hidden browser
panes don't run animation frames or pdf.js, so screenshots can lag and PDFs won't draw.

## Conventions

- Rust: errors as `AppError`; blocking work in `spawn_blocking`; comments say why.
- Generated LaTeX must compile with Tectonic (XeTeX) and pdfLaTeX; add a test, and an
  ignored compile test for new document kinds.
- Linux uses WebKitGTK: check CSS and focus behaviour there (trailing spaces, focus
  after deletes, spell checking are WebKit-specific).
- Windows: external programs start through `nawl_compile::tool()` (no console
  window); executables need `std::env::consts::EXE_SUFFIX`.
