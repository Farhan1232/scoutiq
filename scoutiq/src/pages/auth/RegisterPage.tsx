import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  isAuthMethodEnabled,
  useAuth,
  validateRegisterForm,
  type AuthErrorCode,
  type FieldErrors,
} from "../../auth";
import { AuthLayout } from "../../components/auth/AuthLayout";
import type { AuthScreen } from "../../components/auth/AuthGate";
import { UpcomingAuthMethods } from "../../components/auth/UpcomingAuthMethods";
import { Button } from "../../components/ui/Button";
import { TextField } from "../../components/ui/TextField";

interface RegisterPageProps {
  onNavigate: (screen: AuthScreen) => void;
  onAuthenticated: () => void;
}

export function RegisterPage({
  onNavigate,
  onAuthenticated,
}: RegisterPageProps) {
  const { t } = useTranslation();
  const { signUp } = useAuth();
  const [displayName, setDisplayName] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [errorCode, setErrorCode] = useState<AuthErrorCode | null>(null);
  const [needsConfirmation, setNeedsConfirmation] = useState(false);

  const onSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setErrorCode(null);
    setNeedsConfirmation(false);

    const payload = { displayName, email, password, confirmPassword };
    const errors = validateRegisterForm(payload);
    if (errors) {
      setFieldErrors(errors);
      return;
    }

    setFieldErrors({});
    setBusy(true);
    const result = await signUp(payload);
    setBusy(false);

    if (!result.ok) {
      setErrorCode(result.error.code);
      return;
    }

    if (!result.session) {
      setNeedsConfirmation(true);
      return;
    }

    onAuthenticated();
  };

  return (
    <AuthLayout
      title={t("auth.register.title")}
      subtitle={t("auth.register.subtitle")}
      footer={
        <>
          {t("auth.register.hasAccount")}{" "}
          <button type="button" onClick={() => onNavigate("login")}>
            {t("auth.register.signIn")}
          </button>
        </>
      }
    >
      {errorCode ? (
        <p className="auth-form__error">{t(`auth.errors.${errorCode}`)}</p>
      ) : null}

      {needsConfirmation ? (
        <p className="auth-form__success">{t("auth.register.checkEmail")}</p>
      ) : null}

      {isAuthMethodEnabled("emailPassword") && !needsConfirmation ? (
        <form className="auth-form" onSubmit={(e) => void onSubmit(e)} noValidate>
          <TextField
            label={t("auth.fields.displayName")}
            name="displayName"
            type="text"
            autoComplete="name"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            error={
              fieldErrors.displayName
                ? t(`auth.errors.${fieldErrors.displayName}`)
                : undefined
            }
          />
          <TextField
            label={t("auth.fields.email")}
            name="email"
            type="email"
            autoComplete="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            error={
              fieldErrors.email ? t(`auth.errors.${fieldErrors.email}`) : undefined
            }
          />
          <TextField
            label={t("auth.fields.password")}
            name="password"
            type="password"
            autoComplete="new-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            hint={t("auth.register.passwordHint")}
            error={
              fieldErrors.password
                ? t(`auth.errors.${fieldErrors.password}`)
                : undefined
            }
          />
          <TextField
            label={t("auth.fields.confirmPassword")}
            name="confirmPassword"
            type="password"
            autoComplete="new-password"
            value={confirmPassword}
            onChange={(e) => setConfirmPassword(e.target.value)}
            error={
              fieldErrors.confirmPassword
                ? t(`auth.errors.${fieldErrors.confirmPassword}`)
                : undefined
            }
          />
          <div className="auth-form__actions">
            <Button type="submit" variant="primary" disabled={busy}>
              {busy ? t("auth.working") : t("auth.register.submit")}
            </Button>
          </div>
        </form>
      ) : null}

      <UpcomingAuthMethods />
    </AuthLayout>
  );
}
