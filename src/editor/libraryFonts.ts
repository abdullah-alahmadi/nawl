// Texloom's Arabic font library on screen: each family is loaded from the app (the
// same files the PDF uses) into the page as "tl-<id>", regular and bold.
import * as api from "../api";
import type { FontChoice } from "../bindings/FontChoice";

const loading = new Map<string, Promise<void>>();

/** The CSS family a library font is registered under. */
export const libraryFamily = (id: string) => `tl-${id}`;

/** Loads a library font into the page once (later calls share the first load). */
export function ensureLibraryFont(id: string): Promise<void> {
  let p = loading.get(id);
  if (!p) {
    p = (async () => {
      const [regular, bold] = await Promise.all([api.readLibraryFont(id, false), api.readLibraryFont(id, true)]);
      for (const [data, weight] of [[regular, "400"], [bold, "700"]] as const) {
        const face = new FontFace(libraryFamily(id), data, { weight });
        await face.load();
        document.fonts.add(face);
      }
    })().catch((e) => {
      loading.delete(id);
      console.warn(`font ${id}:`, e);
    });
    loading.set(id, p);
  }
  return p;
}

/** The library fonts a template uses (to load before showing its page). */
export function libraryFontIds(fonts: (FontChoice | null | undefined)[]): string[] {
  return [...new Set(fonts.flatMap((f) => (f?.kind === "library" ? [f.id] : [])))];
}
