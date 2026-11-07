// The interface's language. Strings live in one file per area of the app
// (app.ts, designer.ts…), each with `en` and `ar` tables; `ar` is typed by `en`'s
// keys, so a missing Arabic string fails the type check.
//
// The document's own words (heading labels, "Figure", citations) follow the
// *document's* language, not this one: see editor/language.ts.
import { createContext, useContext } from "react";
import * as app from "./app";
import * as common from "./common";
import * as designer from "./designer";
import * as designerContent from "./designerContent";
import * as dialogs from "./dialogs";
import * as editor from "./editor";

const en = { ...common.en, ...app.en, ...designer.en, ...designerContent.en, ...dialogs.en, ...editor.en };
export type MessageKey = keyof typeof en;
const ar: Record<MessageKey, string> = { ...common.ar, ...app.ar, ...designer.ar, ...designerContent.ar, ...dialogs.ar, ...editor.ar };

export type UiLanguage = "en" | "ar";
const TABLES: Record<UiLanguage, Record<MessageKey, string>> = { en, ar };

/** The interface language to use for a setting ("system" follows the OS). */
export function resolveUiLanguage(setting: string): UiLanguage {
  if (setting === "en" || setting === "ar") return setting;
  return typeof navigator !== "undefined" && navigator.language?.toLowerCase().startsWith("ar") ? "ar" : "en";
}

let current: UiLanguage = "en";

/** Sets the language `t()` uses outside React components (dialogs, errors). */
export function setUiLanguage(lang: UiLanguage) {
  current = lang;
}

export function uiLanguage(): UiLanguage {
  return current;
}

/** A string in `lang`, with {name} placeholders filled in. */
export function translate(lang: UiLanguage, key: MessageKey, vars?: Record<string, string | number>): string {
  let s = TABLES[lang][key] ?? en[key] ?? key;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.split(`{${k}}`).join(String(v));
  return s;
}

/** A string in the current interface language (for code outside components). */
export function t(key: MessageKey, vars?: Record<string, string | number>): string {
  return translate(current, key, vars);
}

export const LanguageContext = createContext<UiLanguage>("en");

/** `t` for components: re-renders them when the language changes. */
export function useT() {
  const lang = useContext(LanguageContext);
  return (key: MessageKey, vars?: Record<string, string | number>) => translate(lang, key, vars);
}
