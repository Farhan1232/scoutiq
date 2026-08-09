import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import "./AuthLayout.css";

interface AuthLayoutProps {
  title: string;
  subtitle: string;
  children: ReactNode;
  footer?: ReactNode;
}

export function AuthLayout({
  title,
  subtitle,
  children,
  footer,
}: AuthLayoutProps) {
  const { t } = useTranslation();

  return (
    <div className="auth-layout">
      <div className="auth-layout__panel">
        <div className="auth-layout__brand">
          <div className="auth-layout__logo" aria-hidden="true">
            <span />
          </div>
          <div>
            <p className="auth-layout__product">{t("app.name")}</p>
            <p className="auth-layout__tagline">{t("auth.tagline")}</p>
          </div>
        </div>

        <header className="auth-layout__header">
          <h1>{title}</h1>
          <p>{subtitle}</p>
        </header>

        <div className="auth-layout__body">{children}</div>

        {footer ? <footer className="auth-layout__footer">{footer}</footer> : null}
      </div>
    </div>
  );
}
