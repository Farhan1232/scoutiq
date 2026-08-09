import { useTranslation } from "react-i18next";
import { useAuth } from "../../auth";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { Icon } from "../icons/Icon";
import "./AccountProfileSection.css";

interface AccountProfileSectionProps {
  onOpenAccountSettings: () => void;
  onRequestSignIn: () => void;
}

export function AccountProfileSection({
  onOpenAccountSettings,
  onRequestSignIn,
}: AccountProfileSectionProps) {
  const { t } = useTranslation();
  const { user, isAuthenticated, signOut } = useAuth();

  if (!isAuthenticated || !user) {
    return (
      <div className="account-profile account-profile--empty">
        <p>{t("settings.account.signedOut")}</p>
        <Button variant="primary" size="sm" onClick={onRequestSignIn}>
          <Icon name="user" size={14} />
          {t("settings.account.signIn")}
        </Button>
      </div>
    );
  }

  return (
    <div className="account-profile">
      <div className="account-profile__identity">
        <span className="account-profile__avatar">{user.initials}</span>
        <div className="account-profile__meta">
          <p className="account-profile__name">{user.displayName}</p>
          <p className="account-profile__email">{user.email}</p>
          <div className="account-profile__badges">
            <Badge tone="accent">{t(`auth.plans.${user.plan}`)}</Badge>
            <Badge tone="neutral">{t(`languages.${user.language}`)}</Badge>
          </div>
        </div>
      </div>

      <div className="account-profile__fields">
        <div className="settings-row">
          <span>{t("auth.fields.displayName")}</span>
          <strong>{user.displayName}</strong>
        </div>
        <div className="settings-row">
          <span>{t("auth.fields.email")}</span>
          <strong>{user.email}</strong>
        </div>
        <div className="settings-row">
          <span>{t("auth.fields.language")}</span>
          <strong>{t(`languages.${user.language}`)}</strong>
        </div>
        <div className="settings-row">
          <span>{t("auth.fields.plan")}</span>
          <strong>{t(`auth.plans.${user.plan}`)}</strong>
        </div>
      </div>

      <div className="account-profile__actions">
        <Button variant="secondary" size="sm" onClick={onOpenAccountSettings}>
          <Icon name="user" size={14} />
          {t("settings.account.manage")}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            void signOut();
          }}
        >
          {t("settings.account.signOut")}
        </Button>
      </div>
    </div>
  );
}
