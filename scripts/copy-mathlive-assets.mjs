// Copies MathLive's fonts into public/ so they're served offline (Vite's dependency
// pre-bundling breaks MathLive's default relative font path).
import { cpSync, existsSync } from "node:fs";

const src = "node_modules/mathlive/fonts";
if (existsSync(src)) {
  cpSync(src, "public/mathlive/fonts", { recursive: true });
}
