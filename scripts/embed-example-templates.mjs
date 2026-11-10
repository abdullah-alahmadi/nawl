// Copies each built-in template into the example documents that use it, as the app
// does when it saves a document: opening an example then switches to its template.
// Run after changing one of these templates: node scripts/embed-example-templates.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

/** Example document → the built-in template it is written for. */
export const EXAMPLES = {
  "arabic-article.loom": "arabic-article",
  "arabic-load-forecasting.loom": "arabic-article",
  "arabic-article-two-column.loom": "arabic-article-two-column",
  "arabic-article-two-column-arabic-only.loom": "arabic-article-two-column-arabic-only",
  "arabic-thesis.loom": "arabic-thesis",
  "arabic-book.loom": "arabic-book",
  "arabic-presentation.loom": "arabic-presentation",
  "arabic-admin-order.loom": "arabic-admin-order",
  "arabic-math.loom": "arabic-article",
};

/** Changes an example makes to its built-in template. */
const CHANGES = {
  // Formulas the Arabic way (right to left).
  "arabic-math.loom": (t) => {
    t.meta.id = "arabic-article-math";
    t.meta.name += " (رياضيات عربية)";
    t.language.math = "arabic";
  },
};

for (const [file, id] of Object.entries(EXAMPLES)) {
  const path = join(root, "examples", file);
  const doc = JSON.parse(readFileSync(path, "utf8"));
  const template = JSON.parse(readFileSync(join(root, "templates", id, "template.json"), "utf8"));
  CHANGES[file]?.(template);
  // The order the app writes: format, version, template, templatePath, then the rest.
  const { format, version, template: _old, templatePath: _path, ...rest } = doc;
  const out = { format, version, template, templatePath: null, ...rest };
  writeFileSync(path, JSON.stringify(out, null, 2) + "\n");
  console.log(`${file}: ${id}`);
}
