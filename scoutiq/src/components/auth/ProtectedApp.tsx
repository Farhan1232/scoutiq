import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useAuth } from "../../auth";

interface ProtectedAppProps {
  children: ReactNode;
  fallback: ReactNode;
}

/**
 * Protects the main ScoutIQ shell behind an authenticated session.
 * Auth screens are rendered via `fallback` while signed out / loading.
 */
export function ProtectedApp({ children, fallback }: ProtectedAppProps) {
  const { t } = useTranslation();
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) {
    return (
      <div className="app-boot">
        <div className="app-boot__mark" aria-hidden="true" />
        <p>{t("app.loading")}</p>
      </div>
    );
  }

  if (!isAuthenticated) {
    return <>{fallback}</>;
  }

  return <>{children}</>;
}
