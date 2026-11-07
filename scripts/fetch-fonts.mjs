#!/usr/bin/env node
// Downloads the Arabic font library (fonts/catalog.json) from github.com/google/fonts
// into src-tauri/fonts, shipped with the app: <id>-Regular.ttf, <id>-Bold.ttf (when
// the family has a bold), and licenses/<id>.txt (every font is under the SIL Open Font
// License, which asks for the license to travel with the fonts). Variable fonts are
// cut into static Regular and Bold files with fonttools (pip install fonttools), since
// XeTeX handles variable fonts poorly.
//
//   node scripts/fetch-fonts.mjs
import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const root = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const out = join(root, "src-tauri", "fonts");
const catalog = JSON.parse(readFileSync(join(root, "fonts", "catalog.json"), "utf8")).fonts;
const python = process.env.PYTHON ?? (process.platform === "win32" ? "python" : "python3");
const base = "https://raw.githubusercontent.com/google/fonts/main/ofl";

mkdirSync(join(out, "licenses"), { recursive: true });
// Fresh files (the README that keeps the folder in git stays).
for (const f of readdirSync(out)) if (f !== "README.md" && f !== "licenses") rmSync(join(out, f), { recursive: true, force: true });

async function download(url, dest) {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`${url}: ${r.status}`);
  writeFileSync(dest, Buffer.from(await r.arrayBuffer()));
}

const work = mkdtempSync(join(tmpdir(), "texloom-fonts-"));
for (const f of catalog) {
  const src = (file) => `${base}/${f.source}/${encodeURIComponent(file)}`;
  if (f.variable) {
    const vf = join(work, `${f.id}.ttf`);
    await download(src(f.variable), vf);
    const axes = Object.entries(f.axes ?? {}).map(([k, v]) => `${k}=${v}`);
    for (const [style, wght] of [["Regular", 400], ["Bold", 700]]) {
      const args = ["-m", "fontTools.varLib.instancer", vf, `wght=${wght}`, ...axes, "--static", "-q", "-o", join(out, `${f.id}-${style}.ttf`)];
      // Named for the cut ("Cairo Bold"), when the font describes its styles (a STAT table).
      try {
        execFileSync(python, [...args, "--update-name-table"], { stdio: "pipe" });
      } catch {
        execFileSync(python, args, { stdio: "inherit" });
      }
    }
  } else {
    await download(src(f.regular), join(out, `${f.id}-Regular.ttf`));
    if (f.bold) await download(src(f.bold), join(out, `${f.id}-Bold.ttf`));
  }
  await download(`${base}/${f.source}/OFL.txt`, join(out, "licenses", `${f.id}.txt`));
  console.log(`ok  ${f.name}`);
}
rmSync(work, { recursive: true, force: true });
const size = readdirSync(out).filter((f) => f.endsWith(".ttf")).reduce((n, f) => n + statSync(join(out, f)).size, 0);
console.log(`${catalog.length} families, ${(size / 1e6).toFixed(1)} MB in ${out}`);
