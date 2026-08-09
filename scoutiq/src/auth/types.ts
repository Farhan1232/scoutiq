export type OAuthProvider = "google" | "apple";

export type LinkedAuthMethod = "email" | OAuthProvider;

export type UserPlan = "free" | "pro" | "elite";

export type UserLanguage = "de" | "en";

export interface AuthUser {
  id: string;
  email: string;
  /** Display name shown in the app (Konto → Name). */
  displayName: string;
  avatarUrl?: string | null;
  initials: string;
  language: UserLanguage;
  plan: UserPlan;
  linkedMethods: LinkedAuthMethod[];
  createdAt: string;
}

export interface AuthSession {
  user: AuthUser;
  accessToken: string;
  refreshToken?: string;
  expiresAt?: string;
}

export type AuthErrorCode =
  | "invalid_credentials"
  | "invalid_email"
  | "invalid_name"
  | "email_taken"
  | "weak_password"
  | "password_mismatch"
  | "user_not_found"
  | "email_not_confirmed"
  | "email_provider_disabled"
  | "not_configured"
  | "oauth_unavailable"
  | "network"
  | "unknown";

export interface AuthError {
  code: AuthErrorCode;
  message: string;
  /** Optional raw provider detail for debugging / fallback UI. */
  detail?: string;
}

export type AuthResult =
  | { ok: true; session: AuthSession | null }
  | { ok: false; error: AuthError };

export interface EmailPasswordCredentials {
  email: string;
  password: string;
}

export interface SignUpCredentials extends EmailPasswordCredentials {
  displayName: string;
  confirmPassword?: string;
}

export interface ProfileUpdateInput {
  displayName?: string;
  avatarUrl?: string | null;
  language?: UserLanguage;
  plan?: UserPlan;
}

export interface FieldErrors {
  displayName?: AuthErrorCode;
  email?: AuthErrorCode;
  password?: AuthErrorCode;
  confirmPassword?: AuthErrorCode;
}

