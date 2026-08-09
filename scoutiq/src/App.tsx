import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAuth } from "./auth";
import { AuthGate, type AuthScreen } from "./components/auth/AuthGate";
import { ProtectedApp } from "./components/auth/ProtectedApp";
import { AppShell } from "./components/layout/AppShell";
import { DashboardPage } from "./pages/DashboardPage";
import { PlaceholderPage } from "./pages/PlaceholderPage";
import { SettingsPage, type SettingsView } from "./pages/SettingsPage";
import { SquadPage } from "./pages/SquadPage";
import { SaveAnalysePage } from "./pages/SaveAnalysePage";
import { EmailCallbackPage } from "./pages/auth/EmailCallbackPage";
import { isAuthCallbackUrl } from "./services/authCallback";
import {
  isDeveloperMode,
  subscribeDeveloperMode,
} from "./dev/developerMode";
import type { NavId } from "./types";

function App() {
  const { t } = useTranslation();
  const { isAuthenticated } = useAuth();
  const [active, setActive] = useState<NavId>("dashboard");
  const [settingsView, setSettingsView] = useState<SettingsView>("overview");
  const [authScreen, setAuthScreen] = useState<AuthScreen>("login");
  const [devMode, setDevMode] = useState(isDeveloperMode);
  const [showEmailCallback, setShowEmailCallback] = useState(() =>
    isAuthCallbackUrl(),
  );
  const title = t(`nav.${active}`);

  useEffect(() => subscribeDeveloperMode(setDevMode), []);

  useEffect(() => {
    if (!isAuthenticated) {
      setActive("dashboard");
      setSettingsView("overview");
      setAuthScreen("login");
    }
  }, [isAuthenticated]);

  useEffect(() => {
    if (active !== "settings") {
      setSettingsView("overview");
    }
  }, [active]);

  useEffect(() => {
    if (!devMode && active === "saveAnalyse") {
      setActive("dashboard");
    }
  }, [devMode, active]);

  if (showEmailCallback) {
    return (
      <EmailCallbackPage
        onContinue={() => {
          setShowEmailCallback(false);
          setAuthScreen("login");
        }}
      />
    );
  }

  return (
    <ProtectedApp
      fallback={
        <AuthGate
          screen={authScreen}
          onNavigate={setAuthScreen}
          onAuthenticated={() => {
            setActive("dashboard");
          }}
        />
      }
    >
      <AppShell
        active={active}
        title={title}
        onNavigate={setActive}
        onOpenProfile={() => {
          setActive("settings");
          setSettingsView("account");
        }}
      >
        {active === "dashboard" ? (
          <DashboardPage />
        ) : active === "squad" ? (
          <SquadPage />
        ) : active === "saveAnalyse" && devMode ? (
          <SaveAnalysePage />
        ) : active === "settings" ? (
          <SettingsPage
            initialView={settingsView}
            onRequestSignIn={() => {
              setAuthScreen("login");
            }}
          />
        ) : active === "saveAnalyse" ? (
          <DashboardPage />
        ) : (
          <PlaceholderPage id={active} title={title} />
        )}
      </AppShell>
    </ProtectedApp>
  );
}

export default App;
