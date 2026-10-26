// UI translations. German source strings are the keys; other languages map them.
// Missing entries fall back to the German text, so the UI never shows raw keys.
import { en } from "./locales/en";

export type Lang = "de" | "en";
export const LANGUAGES: { id: Lang; label: string }[] = [
  { id: "de", label: "Deutsch" },
  { id: "en", label: "English" },
];

const dictionaries: Record<Lang, Record<string, string> | null> = { de: null, en };

let current: Lang = "en";

/** "auto" follows the system/browser language. */
export function resolveLang(setting: string | null | undefined): Lang {
  if (setting === "de" || setting === "en") return setting;
  const sys = (navigator.languages?.[0] || navigator.language || "en").toLowerCase();
  return sys.startsWith("de") ? "de" : "en";
}

export function setLang(lang: Lang) {
  current = lang;
  document.documentElement.lang = lang;
}

export function getLang(): Lang {
  return current;
}

/** Translate a German source string; `{name}` placeholders are filled from `vars`. */
export function t(key: string, vars?: Record<string, string | number>): string {
  const dict = dictionaries[current];
  let s = (dict && dict[key]) || key;
  if (vars) {
    for (const [k, v] of Object.entries(vars)) s = s.split(`{${k}}`).join(String(v));
  }
  return s;
}

export function locale(): string {
  return current === "de" ? "de-DE" : "en-GB";
}
