export {
  AUTH_METHODS,
  FUTURE_AUTH_METHODS,
  isAuthMethodEnabled,
} from "./config";
export { AuthProvider, useAuth } from "../context/AuthContext";
export {
  getSession,
  onAuthStateChange,
  resetPassword,
  signInWithOAuth,
  signInWithPassword,
  signOut,
  signUp,
  updateProfile,
} from "../services/authService";
export {
  validateDisplayName,
  validateEmail,
  validateForgotPasswordForm,
  validateLoginForm,
  validateRegisterForm,
} from "./validation";
export type {
  AuthError,
  AuthErrorCode,
  AuthResult,
  AuthSession,
  AuthUser,
  EmailPasswordCredentials,
  FieldErrors,
  LinkedAuthMethod,
  OAuthProvider,
  ProfileUpdateInput,
  SignUpCredentials,
  UserLanguage,
  UserPlan,
} from "./types";
