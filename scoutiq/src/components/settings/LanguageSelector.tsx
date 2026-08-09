import type { ChangeEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  DEFAULT_LANGUAGE,
  SUPPORTED_LANGUAGES,
  isAppLanguage,
  setAppLanguage,
  type AppLanguage,
} from "../../i18n/i18n";
import "./LanguageSelector.css";

function resolveLanguage(language?: string): AppLanguage {
  const normalized = (language ?? DEFAULT_LANGUAGE).split("-")[0] ?? DEFAULT_LANGUAGE;
  return isAppLanguage(normalized) ? normalized : DEFAULT_LANGUAGE;
}

export function LanguageSelector() {
  const { t, i18n } = useTranslation();
  const current = resolveLanguage(i18n.resolvedLanguage ?? i18n.language);

  const onChange = (event: ChangeEvent<HTMLSelectElement>) => {
    const next = event.target.value;
    if (isAppLanguage(next)) {
      void setAppLanguage(next);
    }
  };

  return (
    <div className="language-selector">
      <label className="language-selector__label" htmlFor="language-select">
        {t("settings.language.label")}
      </label>
      <div className="language-selector__control">
        <select
          id="language-select"
          className="language-selector__select"
          value={current}
          onChange={onChange}
        >
          {SUPPORTED_LANGUAGES.map((code) => (
            <option key={code} value={code}>
              {t(`languages.${code}`)}
            </option>
          ))}
        </select>
      </div>
      <p className="language-selector__hint">{t("settings.language.hint")}</p>
    </div>
  );
}
