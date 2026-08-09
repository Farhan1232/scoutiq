import { useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import {
  useAuth,
  validateForgotPasswordForm,
  type AuthErrorCode,
  type FieldErrors,
} from "../../auth";
import { AuthLayout } from "../../components/auth/AuthLayout";
import type { AuthScreen } from "../../components/auth/AuthGate";
import { Button } from "../../components/ui/Button";
import { TextField } from "../../components/ui/TextField";

interface ForgotPasswordPageProps {
  onNavigate: (screen: AuthScreen) => void;
}

export function ForgotPasswordPage({ onNavigate }: ForgotPasswordPageProps) {
  const { t } = useTranslation();
  const { resetPassword } = useAuth();
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<FieldErrors>({});
  const [errorCode, setErrorCode] = useState<AuthErrorCode | null>(null);
  const [sent, setSent] = useState(false);

  const onSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setErrorCode(null);
    setSent(false);

    const errors = validateForgotPasswordForm(email);
    if (errors) {
      setFieldErrors(errors);
      return;
    }

    setFieldErrors({});
    setBusy(true);
    const result = await resetPassword(email);
    setBusy(false);

    if (!result.ok) {
      setErrorCode(result.error.code);
      return;
    }

    setSent(true);
  };

  return (
    <AuthLayout
      title={t("auth.forgot.title")}
      subtitle={t("auth.forgot.subtitle")}
      footer={
        <>
          {t("auth.forgot.remembered")}{" "}
          <button type="button" onClick={() => onNavigate("login")}>
            {t("auth.forgot.backToLogin")}
          </button>
        </>
      }
    >
      {errorCode ? (
        <p className="auth-form__error">{t(`auth.errors.${errorCode}`)}</p>
      ) : null}

      {sent ? (
        <p className="auth-form__success">{t("auth.forgot.success")}</p>
      ) : null}

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
        <div className="auth-form__actions">
          <Button type="submit" variant="primary" disabled={busy}>
            {busy ? t("auth.working") : t("auth.forgot.submit")}
          </Button>
        </div>
      </form>
    </AuthLayout>
  );
}
