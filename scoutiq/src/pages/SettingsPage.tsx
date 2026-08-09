import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { CLUB } from "../data/mock";
import {
  isDeveloperMode,
  setDeveloperMode,
  subscribeDeveloperMode,
} from "../dev/developerMode";
import { AccountProfileSection } from "../components/settings/AccountProfileSection";
import { CloudSection } from "../components/settings/CloudSection";
import { LanguageSelector } from "../components/settings/LanguageSelector";
import { ThemeSelector } from "../components/settings/ThemeSelector";
import { UpdatesSection } from "../components/settings/UpdatesSection";
import { AccountSettingsPage } from "./auth/AccountSettingsPage";
import "./SettingsPage.css";

export type SettingsView = "overview" | "account";

interface SettingsPageProps {
  onRequestSignIn: () => void;
  initialView?: SettingsView;
}

export function SettingsPage({
  onRequestSignIn,
  initialView = "overview",
}: SettingsPageProps) {
  const { t } = useTranslation();
  const [view, setView] = useState<SettingsView>(initialView);
  const [devMode, setDevMode] = useState(isDeveloperMode);

  useEffect(() => {
    setView(initialView);
  }, [initialView]);

  useEffect(() => subscribeDeveloperMode(setDevMode), []);

  if (view === "account") {
    return <AccountSettingsPage onBack={() => setView("overview")} />;
  }

  return (
    <div className="settings">
      <header className="settings__intro">
        <h2>{t("settings.title")}</h2>
        <p>{t("settings.subtitle")}</p>
      </header>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("settings.account.title")}</h3>
          <p>{t("settings.account.description")}</p>
        </div>
        <AccountProfileSection
          onOpenAccountSettings={() => setView("account")}
          onRequestSignIn={onRequestSignIn}
        />
      </section>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("settings.cloud.title")}</h3>
          <p>{t("settings.cloud.description")}</p>
        </div>
        <CloudSection />
      </section>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("settings.language.title")}</h3>
          <p>{t("settings.language.description")}</p>
        </div>
        <LanguageSelector />
      </section>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("settings.theme.title")}</h3>
          <p>{t("settings.theme.description")}</p>
        </div>
        <ThemeSelector />
      </section>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("settings.developer.title")}</h3>
          <p>{t("settings.developer.description")}</p>
        </div>
        <div className="settings-rows">
          <label className="settings-row">
            <span>{t("settings.developer.enable")}</span>
            <input
              type="checkbox"
              checked={devMode}
              onChange={(e) => {
                setDeveloperMode(e.target.checked);
                setDevMode(e.target.checked);
              }}
            />
          </label>
        </div>
      </section>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("settings.updates.title")}</h3>
          <p>{t("settings.updates.description")}</p>
        </div>
        <UpdatesSection />
      </section>

      <section className="settings-card">
        <div className="settings-card__head">
          <h3>{t("settings.club.title")}</h3>
          <p>{t("settings.club.description")}</p>
        </div>
        <div className="settings-rows">
          <div className="settings-row">
            <span>{t("settings.club.clubLabel")}</span>
            <strong>{t("club.name")}</strong>
          </div>
          <div className="settings-row">
            <span>{t("settings.club.seasonLabel")}</span>
            <strong>{CLUB.season}</strong>
          </div>
          <div className="settings-row">
            <span>{t("settings.club.managerLabel")}</span>
            <strong>{t("club.manager")}</strong>
          </div>
        </div>
      </section>
    </div>
  );
}
