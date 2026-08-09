import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  isAuthMethodEnabled,
  useAuth,
  validateLoginForm,
  type AuthErrorCode,
  type FieldErrors,
} from "../../auth";
import { AuthLayout } from "../../components/auth/AuthLayout";
import { UpcomingAuthMethods } from "../../components/auth/UpcomingAuthMethods";
import type { AuthScreen } from "../../components/auth/AuthGate";
import { Button } from "../../components/ui/Button";
import { TextField } from "../../components/ui/TextField";

interface LoginPageProps {
  onNavigate: (screen: AuthScreen) => void;
  onAuthenticated: () => void;
}

export function LoginPage({ onNavigate, onAuthenticated }: LoginPageProps) {
  const { t } = useTranslation();
  const { signInWithPassword } = useAuth();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [errorCode, setErrorCode] = useState<AuthErrorCode | null>(null);

  const onSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setErrorCode(null);

    const errors = validateLoginForm({ email, password });
    if (errors) {
      setFieldErrors(errors);
      return;
    }

    setFieldErrors({});
    setBusy(true);
    try {
      const result = await signInWithPassword({ email, password });
      if (!result.ok) {
        setErrorCode(result.error.code);
        return;
      }
      onAuthenticated();
    } catch {
      setErrorCode("unknown");
    } finally {
      setBusy(false);
    }
  };

  return (
    <AuthLayout
      title={t("auth.login.title")}
      subtitle={t("auth.login.subtitle")}
      footer={
        <>
          {t("auth.login.noAccount")}{" "}
          <button type="button" onClick={() => onNavigate("register")}>
            {t("auth.login.createAccount")}
          </button>
        </>
      }
    >
      {errorCode ? (
        <p className="auth-form__error">{t(`auth.errors.${errorCode}`)}</p>
      ) : null}

      {isAuthMethodEnabled("emailPassword") ? (
        <form className="auth-form" onSubmit={(e) => void onSubmit(e)} noValidate>
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
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            error={
              fieldErrors.password
                ? t(`auth.errors.${fieldErrors.password}`)
                : undefined
            }
          />
          <button
            type="button"
            className="auth-form__link"
            onClick={() => onNavigate("forgot-password")}
          >
            {t("auth.login.forgotPassword")}
          </button>
          <div className="auth-form__actions">
            <Button type="submit" variant="primary" disabled={busy}>
              {busy ? t("auth.working") : t("auth.login.submit")}
            </Button>
          </div>
        </form>
      ) : null}

      <UpcomingAuthMethods />
    </AuthLayout>
  );
}
