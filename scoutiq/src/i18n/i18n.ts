import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import de from "./locales/de.json";
import en from "./locales/en.json";

export const SUPPORTED_LANGUAGES = ["de", "en"] as const;
export type AppLanguage = (typeof SUPPORTED_LANGUAGES)[number];

export const LANGUAGE_STORAGE_KEY = "scoutiq.language";
export const DEFAULT_LANGUAGE: AppLanguage = "de";

export function isAppLanguage(value: string): value is AppLanguage {
  return (SUPPORTED_LANGUAGES as readonly string[]).includes(value);
}

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      de: { translation: de },
      en: { translation: en },
    },
    fallbackLng: DEFAULT_LANGUAGE,
    supportedLngs: [...SUPPORTED_LANGUAGES],
    nonExplicitSupportedLngs: true,
    load: "languageOnly",
    interpolation: {
      escapeValue: false,
    },
    detection: {
      order: ["localStorage"],
      caches: ["localStorage"],
      lookupLocalStorage: LANGUAGE_STORAGE_KEY,
    },
    react: {
      useSuspense: false,
    },
  });

const syncDocumentLanguage = (language: string) => {
  const normalized = language.split("-")[0] ?? DEFAULT_LANGUAGE;
  document.documentElement.lang = isAppLanguage(normalized)
    ? normalized
    : DEFAULT_LANGUAGE;
};

syncDocumentLanguage(i18n.resolvedLanguage ?? i18n.language);
i18n.on("languageChanged", syncDocumentLanguage);

export async function setAppLanguage(language: AppLanguage) {
  await i18n.changeLanguage(language);
  window.localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
}

export default i18n;
