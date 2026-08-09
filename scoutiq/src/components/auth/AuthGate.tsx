import { ForgotPasswordPage } from "../../pages/auth/ForgotPasswordPage";
import { LoginPage } from "../../pages/auth/LoginPage";
import { RegisterPage } from "../../pages/auth/RegisterPage";

export type AuthScreen = "login" | "register" | "forgot-password";

interface AuthGateProps {
  screen: AuthScreen;
  onNavigate: (screen: AuthScreen) => void;
  onAuthenticated: () => void;
}

export function AuthGate({
  screen,
  onNavigate,
  onAuthenticated,
}: AuthGateProps) {
  if (screen === "register") {
    return (
      <RegisterPage
        onNavigate={onNavigate}
        onAuthenticated={onAuthenticated}
      />
    );
  }

  if (screen === "forgot-password") {
    return <ForgotPasswordPage onNavigate={onNavigate} />;
  }

  return (
    <LoginPage onNavigate={onNavigate} onAuthenticated={onAuthenticated} />
  );
}
