// Texloom's Arabic font library on screen: each family is loaded from the app (the
// same files the PDF uses) into the page as "tl-<id>", regular and bold.
import * as api from "../api";
import type { FontChoice } from "../bindings/FontChoice";
import type { Template } from "../bindings/Template";

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

/** Every font a template uses: the text fonts, heading levels' and title items'. */
export function templateFonts(t: Template): (FontChoice | undefined)[] {
  const items = [t.titlePage.items, ...t.frontMatter.flatMap((f) => (f.type === "fixedPage" ? [f.items] : []))].flat();
  return [
    t.fonts.main,
    t.fonts.sans,
    t.fonts.mono,
    t.fonts.arabic,
    // The variables of Arabic formulas (Aref Ruqaa unless the template says otherwise).
    t.language?.main === "arabic" && t.language.math === "arabic" ? (t.fonts.arabicMath ?? { kind: "library", id: "aref-ruqaa" }) : undefined,
    ...t.headings.levels.map((h) => h.text.font),
    ...items.map((i) => (i.content.type === "text" ? i.content.style.font : undefined)),
  ];
}

let libraryList: Promise<api.LibraryFont[]> | null = null;

/** The font library's families (asked for once). */
export function libraryFonts(): Promise<api.LibraryFont[]> {
  libraryList ??= api.listLibraryFonts().catch(() => []);
  return libraryList;
}

let arabicSystemList: Promise<string[]> | null = null;

/** Installed fonts with Arabic letters (asked for once: it reads the font files). */
export function arabicSystemFonts(): Promise<string[]> {
  arabicSystemList ??= api.listArabicSystemFonts().catch(() => []);
  return arabicSystemList;
}
