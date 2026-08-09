import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import { Icon } from "../icons/Icon";
import { useUpdates } from "../../updates";
import "./UpdatesSection.css";

export function UpdatesSection() {
  const { t } = useTranslation();
  const {
    version,
    status,
    messageKey,
    messageParams,
    availableVersion,
    isChecking,
    checkForUpdates,
  } = useUpdates();

  return (
    <div className="updates-section">
      <div className="settings-row">
        <span>{t("settings.updates.currentVersion")}</span>
        <strong className="updates-section__version">v{version}</strong>
      </div>

      <div className="updates-section__actions">
        <Button
          variant="secondary"
          size="sm"
          onClick={() => {
            void checkForUpdates();
          }}
          disabled={isChecking}
        >
          <Icon name="sparkles" size={14} />
          {isChecking
            ? t("settings.updates.checking")
            : t("settings.updates.checkButton")}
        </Button>
      </div>

      <div
        className={`updates-status updates-status--${status}`}
        role="status"
        aria-live="polite"
      >
        <span className="updates-status__dot" aria-hidden="true" />
        <div className="updates-status__copy">
          <p className="updates-status__label">
            {t("settings.updates.statusLabel")}
          </p>
          <p className="updates-status__message">
            {t(messageKey, messageParams)}
            {availableVersion ? (
              <span className="updates-status__available">
                {" "}
                ({availableVersion})
              </span>
            ) : null}
          </p>
        </div>
      </div>
    </div>
  );
}
