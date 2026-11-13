#!/usr/bin/env node
// Fills a Tectonic package cache with everything Nawl's built-in templates and
// beamer theme previews need, so the installed app compiles them without internet
// (src-tauri/tex-cache, shipped as a resource; see nawl_compile::set_cache_dir).
//
//   scripts/fetch-tectonic.sh && node scripts/build-tex-cache.mjs
//
// Needs the Tectonic sidecar (scripts/fetch-tectonic.sh) and builds nawl-cli.
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const cache = join(root, "src-tauri", "tex-cache");
const windows = process.platform === "win32";
const tectonic = readdirSync(join(root, "src-tauri", "binaries"))
  .filter((f) => f.startsWith("nawl-tectonic-"))
  .map((f) => join(root, "src-tauri", "binaries", f))[0];
if (!tectonic) throw new Error("No Tectonic in src-tauri/binaries: run scripts/fetch-tectonic.sh first");

// A fresh cache (the README that keeps the folder in git stays).
mkdirSync(cache, { recursive: true });
for (const f of readdirSync(cache)) if (f !== "README.md") rmSync(join(cache, f), { recursive: true, force: true });
const env = { ...process.env, TECTONIC_CACHE_DIR: cache, NAWL_TECTONIC: tectonic };

execFileSync("cargo", ["build", "--release", "-p", "nawl-cli"], { cwd: root, stdio: "inherit" });
const cli = join(root, "target", "release", "nawl-cli" + (windows ? ".exe" : ""));
const work = mkdtempSync(join(tmpdir(), "nawl-cache-"));
let failed = 0;

// Each built-in template with a document that uses it. Arabic slides need LuaLaTeX
// (an installed TeX), not Tectonic, so they're left out.
const exampleFor = (id, t) => {
  if (t.kind === "presentation") return t.language?.main === "arabic" ? null : "presentation.nawl";
  if (t.language?.main === "arabic") return "arabic-article.nawl";
  if (t.base) return "journal-article.nawl";
  return t.structure?.topLevel === "section" ? "article.nawl" : "senior-design-report.nawl";
};
for (const id of readdirSync(join(root, "templates"))) {
  const file = join(root, "templates", id, "template.json");
  if (!existsSync(file)) continue;
  const example = exampleFor(id, JSON.parse(readFileSync(file, "utf8")));
  if (!example) continue;
  try {
    execFileSync(cli, ["render", join(root, "examples", example), "--template", file, "-o", join(work, `${id}.pdf`)], { env, stdio: "pipe" });
    console.log(`ok  ${id} (${example})`);
  } catch (e) {
    failed++;
    console.log(`FAIL ${id}: ${String(e.stderr ?? e).slice(-400)}`);
  }
}

// Every theme, colour theme and font theme the picker previews.
const style = readFileSync(join(root, "src", "editor", "templateStyle.ts"), "utf8");
const list = (name) => JSON.parse(style.match(new RegExp(`export const ${name} = (\\[[^\\]]*\\])`))[1].replace(/,\s*\]/, "]"));
const decks = [
  ...list("BEAMER_THEMES").map((x) => [`theme-${x}`, `\\usetheme{${x}}`]),
  ...list("BEAMER_COLOR_THEMES").map((x) => [`color-${x}`, `\\usecolortheme{${x}}`]),
  ...list("BEAMER_FONT_THEMES").map((x) => [`font-${x}`, `\\usefonttheme{${x}}`]),
];
for (const [name, use] of decks) {
  const tex = `\\documentclass{beamer}\n${use}\n\\title{T}\\author{A}\n\\begin{document}\n\\frame{\\titlepage}\n\\begin{frame}{F}\\begin{block}{B}x\\end{block}\\begin{alertblock}{A}y\\end{alertblock}\\end{frame}\n\\end{document}\n`;
  writeFileSync(join(work, `${name}.tex`), tex);
  try {
    execFileSync(tectonic, ["-X", "compile", "--untrusted", `${name}.tex`], { cwd: work, env, stdio: "pipe" });
  } catch (e) {
    failed++;
    console.log(`FAIL ${name}: ${String(e.stderr ?? e).slice(-300)}`);
  }
}
console.log(`themes: ${decks.length} decks`);

rmSync(work, { recursive: true, force: true });
const size = (dir) => readdirSync(dir).reduce((n, f) => {
  const p = join(dir, f);
  return n + (statSync(p).isDirectory() ? size(p) : statSync(p).size);
}, 0);
// The app copies the cache again when this changes (a new build of it).
writeFileSync(join(cache, "texloom-cache-version"), new Date().toISOString() + "\n");
console.log(`cache: ${(size(cache) / 1e6).toFixed(1)} MB in ${cache}`);
if (failed) {
  console.error(`${failed} compiles failed`);
  process.exit(1);
}
