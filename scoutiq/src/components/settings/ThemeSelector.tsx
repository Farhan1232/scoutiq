import { useTranslation } from "react-i18next";
import { Icon } from "../icons/Icon";
import { useTheme } from "../../theme/ThemeProvider";
import {
  THEME_PREFERENCES,
  type ThemePreference,
} from "../../theme/theme";
import type { IconName } from "../../types";
import "./ThemeSelector.css";

const THEME_ICONS: Record<ThemePreference, IconName> = {
  dark: "moon",
  light: "sun",
  system: "monitor",
};

export function ThemeSelector() {
  const { t } = useTranslation();
  const { preference, resolvedTheme, setPreference } = useTheme();

  return (
    <div className="theme-selector">
      <p className="theme-selector__label" id="theme-selector-label">
        {t("settings.theme.label")}
      </p>

      <div
        className="theme-selector__options"
        role="radiogroup"
        aria-labelledby="theme-selector-label"
      >
        {THEME_PREFERENCES.map((option) => {
          const selected = preference === option;
          return (
            <button
              key={option}
              type="button"
              role="radio"
              aria-checked={selected}
              className={`theme-option${selected ? " is-selected" : ""}`}
              onClick={() => setPreference(option)}
            >
              <span className="theme-option__icon">
                <Icon name={THEME_ICONS[option]} size={16} />
              </span>
              <span className="theme-option__copy">
                <span className="theme-option__title">
                  {t(`settings.theme.options.${option}.title`)}
                </span>
                <span className="theme-option__desc">
                  {t(`settings.theme.options.${option}.description`)}
                </span>
              </span>
            </button>
          );
        })}
      </div>

      <p className="theme-selector__hint">
        {t("settings.theme.hint", {
          resolved: t(`settings.theme.resolved.${resolvedTheme}`),
        })}
      </p>
    </div>
  );
}
