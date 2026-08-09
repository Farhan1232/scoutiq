import { useTranslation } from "react-i18next";
import { FUTURE_AUTH_METHODS } from "../../auth";
import { Icon } from "../icons/Icon";
import { Badge } from "../ui/Badge";
import "./SocialAuthButtons.css";

/**
 * Future OAuth providers (Google / Apple) — UI only, not connected.
 */
export function UpcomingAuthMethods() {
  const { t } = useTranslation();

  return (
    <div className="social-auth">
      <div className="social-auth__divider">
        <span>{t("auth.orContinueWith")}</span>
      </div>

      <div className="social-auth__buttons">
        {FUTURE_AUTH_METHODS.map((provider) => (
          <button
            key={provider}
            type="button"
            className="social-auth__upcoming"
            disabled
            aria-disabled="true"
          >
            <Icon name={provider} size={16} />
            <span>{t(`auth.providers.${provider}`)}</span>
            <Badge tone="neutral">{t("auth.comingSoon")}</Badge>
          </button>
        ))}
      </div>

      <p className="social-auth__note">{t("auth.oauthPrepared")}</p>
    </div>
  );
}
