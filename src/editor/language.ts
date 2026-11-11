// The document's language as the editor shows it: direction, digits, the words
// Nawl writes itself ("Chapter", "et al."), and fonts for Arabic text. Mirrors
// crates/codegen/src/language.rs and strings.rs, so the page matches the PDF.
import type { Template } from "../api";
import type { Language } from "../bindings/Language";
import type { Digits } from "../bindings/Digits";

export interface DocLanguage {
  main: Language;
  digits: Digits;
  /** Right to left. */
  rtl: boolean;
  /** BCP 47 tag for `lang` (spell checking). */
  tag: string;
}

export function docLanguage(t: Template | null | undefined): DocLanguage {
  const main = t?.language?.main ?? "english";
  return { main, digits: t?.language?.digits ?? "western", rtl: main === "arabic", tag: main === "arabic" ? "ar" : "en" };
}

const ABJAD: [number, string][] = [
  [1000, "غ"], [900, "ظ"], [800, "ض"], [700, "ذ"], [600, "خ"], [500, "ث"], [400, "ت"], [300, "ش"], [200, "ر"], [100, "ق"],
  [90, "ص"], [80, "ف"], [70, "ع"], [60, "س"], [50, "ن"], [40, "م"], [30, "ل"], [20, "ك"], [10, "ي"],
  [9, "ط"], [8, "ح"], [7, "ز"], [6, "و"], [5, "ه"], [4, "د"], [3, "ج"], [2, "ب"], [1, "ا"],
];

/** Abjad numerals, as polyglossia prints LaTeX's letter counters in Arabic:
 *  ا ب ج د ه و ز ح ط ي, then يا (11), ك (20)… */
export function abjad(n: number): string {
  let out = "";
  for (const [value, letter] of ABJAD) {
    while (n >= value) {
      out += letter;
      n -= value;
    }
  }
  return out;
}

/** The same as a CSS counter style, for list markers and appendix labels. */
export const ABJAD_COUNTER_STYLE = `system:additive;additive-symbols:${ABJAD.map(([v, l]) => `${v} "${l}"`).join(",")}`;

/** Western digits written the document's way (٣٫١ stays ASCII-punctuated: "٣.١"). */
export function localDigits(s: string, digits: Digits): string {
  if (digits !== "arabicIndic") return s;
  return s.replace(/[0-9]/g, (d) => String.fromCharCode(0x0660 + Number(d)));
}

export interface Words {
  chapter: string;
  section: string;
  appendix: string;
  and: string;
  etAl: string;
  noDate: string;
}

const WORDS: Record<Language, Words> = {
  english: { chapter: "Chapter", section: "Section", appendix: "Appendix", and: "and", etAl: "et al.", noDate: "n.d." },
  arabic: { chapter: "الفصل", section: "القسم", appendix: "الملحق", and: "و", etAl: "وآخرون", noDate: "د.ت." },
};

export const words = (lang: Language): Words => WORDS[lang];

/** Arabic letters in a string. */
export const hasArabic = (s: string) => /[؀-ۿݐ-ݿࢠ-ࣿﭐ-﷿ﹰ-﻿]/.test(s);

/** Fonts that show Arabic, after the template's own Arabic font: Amiri (the PDF's
 *  default), then fonts Windows, macOS and Linux usually have. */
export const ARABIC_FALLBACKS = ['"Amiri"', '"Traditional Arabic"', '"Noto Naskh Arabic"', '"Geeza Pro"', '"Simplified Arabic"', '"Arial"'];
