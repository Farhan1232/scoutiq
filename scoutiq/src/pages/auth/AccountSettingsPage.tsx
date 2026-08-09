import { useEffect, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  useAuth,
  validateDisplayName,
  type AuthErrorCode,
  type UserLanguage,
} from "../../auth";
import { setAppLanguage } from "../../i18n/i18n";
import { Badge } from "../../components/ui/Badge";
import { Button } from "../../components/ui/Button";
import { TextField } from "../../components/ui/TextField";
import { Icon } from "../../components/icons/Icon";
import "./AccountSettingsPage.css";

interface AccountSettingsPageProps {
  onBack: () => void;
}

export function AccountSettingsPage({ onBack }: AccountSettingsPageProps) {
  const { t, i18n } = useTranslation();
  const { user, updateProfile } = useAuth();
  const [displayName, setDisplayName] = useState(user?.displayName ?? "");
  const [language, setLanguage] = useState<UserLanguage>(user?.language ?? "de");
  const [busy, setBusy] = useState(false);
  const [nameError, setNameError] = useState<AuthErrorCode | null>(null);
  const [errorCode, setErrorCode] = useState<AuthErrorCode | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    setDisplayName(user?.displayName ?? "");
    setLanguage(user?.language ?? "de");
  }, [user?.displayName, user?.language]);

  if (!user) {
    return null;
  }

  const onSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setErrorCode(null);
    setSaved(false);

    const invalidName = validateDisplayName(displayName);
    if (invalidName) {
      setNameError(invalidName);
      return;
    }

    setNameError(null);
    setBusy(true);
    const result = await updateProfile({ displayName, language });
    setBusy(false);

    if (!result.ok) {
      setErrorCode(result.error.code);
      return;
    }

    if (language !== i18n.language) {
      await setAppLanguage(language);
    }

    setSaved(true);
  };

  return (
    <div className="account-settings">
      <button type="button" className="account-settings__back" onClick={onBack}>
        ← {t("auth.account.back")}
      </button>

      <header className="account-settings__intro">
        <h2>{t("auth.account.title")}</h2>
        <p>{t("auth.account.subtitle")}</p>
      </header>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("auth.account.profileTitle")}</h3>
          <p>{t("auth.account.profileDescription")}</p>
        </div>

        <div className="account-settings__identity">
          <span className="account-settings__avatar">{user.initials}</span>
          <div>
            <p className="account-settings__name">{user.displayName}</p>
            <p className="account-settings__email">{user.email}</p>
          </div>
        </div>

        {errorCode ? (
          <p className="auth-form__error">{t(`auth.errors.${errorCode}`)}</p>
        ) : null}
        {saved ? (
          <p className="auth-form__success">{t("auth.account.saved")}</p>
        ) : null}

        <form className="auth-form" onSubmit={(e) => void onSubmit(e)} noValidate>
          <TextField
            label={t("auth.fields.displayName")}
            name="displayName"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            error={nameError ? t(`auth.errors.${nameError}`) : undefined}
          />
          <TextField
            label={t("auth.fields.email")}
            name="email"
            value={user.email}
            disabled
            hint={t("auth.account.emailLocked")}
          />

          <label className="text-field" htmlFor="account-language">
            <span className="text-field__label">{t("auth.fields.language")}</span>
            <select
              id="account-language"
              className="text-field__input account-settings__select"
              value={language}
              onChange={(e) => setLanguage(e.target.value as UserLanguage)}
            >
              <option value="de">{t("languages.de")}</option>
              <option value="en">{t("languages.en")}</option>
            </select>
          </label>

          <div className="settings-row account-settings__plan-row">
            <span>{t("auth.fields.plan")}</span>
            <strong>{t(`auth.plans.${user.plan}`)}</strong>
          </div>

          <div className="auth-form__actions">
            <Button type="submit" variant="primary" disabled={busy}>
              {busy ? t("auth.working") : t("auth.account.save")}
            </Button>
          </div>
        </form>
      </section>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("auth.account.methodsTitle")}</h3>
          <p>{t("auth.account.methodsDescription")}</p>
        </div>

        <ul className="account-methods">
          {user.linkedMethods.map((method) => (
            <li key={method} className="account-methods__item">
              <span className="account-methods__icon">
                <Icon name={method === "email" ? "mail" : method} size={16} />
              </span>
              <span>
                {t(`auth.providers.${method === "email" ? "email" : method}`)}
              </span>
              <Badge tone="accent">{t("auth.account.connected")}</Badge>
            </li>
          ))}
          <li className="account-methods__item is-muted">
            <span className="account-methods__icon">
              <Icon name="google" size={16} />
            </span>
            <span>{t("auth.providers.google")}</span>
            <Badge tone="neutral">{t("auth.comingSoon")}</Badge>
          </li>
          <li className="account-methods__item is-muted">
            <span className="account-methods__icon">
              <Icon name="apple" size={16} />
            </span>
            <span>{t("auth.providers.apple")}</span>
            <Badge tone="neutral">{t("auth.comingSoon")}</Badge>
          </li>
        </ul>

        <p className="account-settings__hint">{t("auth.account.futureProviders")}</p>
      </section>
    </div>
  );
}
