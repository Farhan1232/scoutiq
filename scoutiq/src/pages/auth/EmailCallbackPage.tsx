import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AuthLayout } from "../../components/auth/AuthLayout";
import { Button } from "../../components/ui/Button";
import {
  handleAuthCallback,
  type AuthCallbackResult,
} from "../../services/authCallback";

interface EmailCallbackPageProps {
  onContinue: () => void;
}

export function EmailCallbackPage({ onContinue }: EmailCallbackPageProps) {
  const { t } = useTranslation();
  const [result, setResult] = useState<AuthCallbackResult | null>(null);

  useEffect(() => {
    let active = true;
    void (async () => {
      const next = await handleAuthCallback();
      if (active) setResult(next);
    })();
    return () => {
      active = false;
    };
  }, []);

  const isError = result?.kind === "error";
  const isReady = result !== null && result.kind !== "none";

  return (
    <AuthLayout
      title={t("auth.callback.title")}
      subtitle={t("auth.callback.subtitle")}
      footer={
        <button type="button" onClick={onContinue}>
          {t("auth.callback.backToLogin")}
        </button>
      }
    >
      {!isReady ? (
        <p className="auth-form__success">{t("auth.callback.working")}</p>
      ) : isError ? (
        <p className="auth-form__error">
          {t(result.messageKey)}
          {result.detail ? ` (${result.detail})` : ""}
        </p>
      ) : (
        <p className="auth-form__success">{t(result.messageKey)}</p>
      )}

      <div className="auth-form__actions">
        <Button type="button" variant="primary" onClick={onContinue}>
          {t("auth.callback.backToLogin")}
        </Button>
      </div>
    </AuthLayout>
  );
}
