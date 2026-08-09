import type { Session, User, AuthError as SupabaseAuthError } from "@supabase/supabase-js";
import {
  getSupabaseRedirectUrl,
  hasSupabaseConfig,
  supabase,
} from "../lib/supabase";
import type {
  AuthError,
  AuthErrorCode,
  AuthResult,
  AuthSession,
  AuthUser,
  EmailPasswordCredentials,
  LinkedAuthMethod,
  ProfileUpdateInput,
  SignUpCredentials,
  UserLanguage,
  UserPlan,
} from "../auth/types";
import {
  validateEmail,
  validateLoginForm,
  validateRegisterForm,
} from "../auth/validation";
import {
  ensureProfile,
  getProfile,
  updateProfileRow,
  type ProfileRow,
} from "./profileService";
import { readStoredThemePreference } from "../theme/theme";

function initialsFromName(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "SQ";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return `${parts[0][0]}${parts[1][0]}`.toUpperCase();
}

function isUserLanguage(value: unknown): value is UserLanguage {
  return value === "de" || value === "en";
}

function isUserPlan(value: unknown): value is UserPlan {
  return value === "free" || value === "pro" || value === "elite";
}

function fail(
  code: AuthErrorCode,
  message?: string,
  detail?: string,
): AuthResult {
  const error: AuthError = {
    code,
    message: message ?? code,
    detail,
  };
  return { ok: false, error };
}

function mapSupabaseAuthError(error: SupabaseAuthError | null): AuthErrorCode {
  if (!error) return "unknown";

  const message = (error.message ?? "").toLowerCase();
  const status = error.status;
  const code = (error as SupabaseAuthError & { code?: string }).code ?? "";

  if (
    code === "email_provider_disabled" ||
    message.includes("email logins are disabled") ||
    message.includes("email provider is disabled")
  ) {
    return "email_provider_disabled";
  }
  if (
    code === "invalid_credentials" ||
    message.includes("invalid login credentials")
  ) {
    return "invalid_credentials";
  }
  if (
    code === "email_not_confirmed" ||
    message.includes("email not confirmed")
  ) {
    return "email_not_confirmed";
  }
  if (
    code === "user_already_exists" ||
    message.includes("user already registered") ||
    message.includes("already been registered")
  ) {
    return "email_taken";
  }
  if (
    code === "weak_password" ||
    (message.includes("password") && message.includes("at least"))
  ) {
    return "weak_password";
  }
  if (code === "validation_failed" || message.includes("invalid email")) {
    return "invalid_email";
  }
  if (
    message.includes("fetch failed") ||
    message.includes("network") ||
    error.name === "AuthRetryableFetchError" ||
    status === 0 ||
    status === 429
  ) {
    return "network";
  }

  return "unknown";
}

function linkedMethodsFromUser(user: User): LinkedAuthMethod[] {
  const providers = new Set(
    (user.identities ?? []).map((identity) => identity.provider),
  );
  const methods: LinkedAuthMethod[] = [];
  if (providers.has("email") || user.email) methods.push("email");
  if (providers.has("google")) methods.push("google");
  if (providers.has("apple")) methods.push("apple");
  return methods.length > 0 ? methods : ["email"];
}

function authUserFromSources(
  user: User,
  profile: ProfileRow | null,
): AuthUser {
  const metaName =
    typeof user.user_metadata?.display_name === "string"
      ? user.user_metadata.display_name
      : typeof user.user_metadata?.full_name === "string"
        ? user.user_metadata.full_name
        : "";
  const displayName =
    profile?.display_name?.trim() ||
    metaName.trim() ||
    user.email?.split("@")[0] ||
    "ScoutIQ";

  const language = isUserLanguage(profile?.language)
    ? profile.language
    : "de";
  const plan = isUserPlan(profile?.plan) ? profile.plan : "free";

  return {
    id: user.id,
    email: user.email ?? "",
    displayName,
    avatarUrl:
      typeof user.user_metadata?.avatar_url === "string"
        ? user.user_metadata.avatar_url
        : null,
    initials: initialsFromName(displayName),
    language,
    plan,
    linkedMethods: linkedMethodsFromUser(user),
    createdAt: profile?.created_at ?? user.created_at,
  };
}

async function buildAuthSession(session: Session): Promise<AuthSession> {
  const profile = await getProfile(session.user.id);
  return {
    user: authUserFromSources(session.user, profile),
    accessToken: session.access_token,
    refreshToken: session.refresh_token,
    expiresAt: session.expires_at
      ? new Date(session.expires_at * 1000).toISOString()
      : undefined,
  };
}

function ensureConfigured(): AuthResult | null {
  if (!hasSupabaseConfig()) {
    return fail("not_configured");
  }
  return null;
}

export async function getSession(): Promise<AuthSession | null> {
  if (!hasSupabaseConfig()) return null;

  const { data, error } = await supabase.auth.getSession();
  if (error || !data.session) return null;

  try {
    return await buildAuthSession(data.session);
  } catch {
    return null;
  }
}

export function onAuthStateChange(
  callback: (session: AuthSession | null) => void,
): () => void {
  if (!hasSupabaseConfig()) {
    return () => undefined;
  }

  const { data } = supabase.auth.onAuthStateChange((_event, session) => {
    void (async () => {
      if (!session) {
        callback(null);
        return;
      }
      try {
        callback(await buildAuthSession(session));
      } catch {
        callback(null);
      }
    })();
  });

  return () => {
    data.subscription.unsubscribe();
  };
}

export async function signInWithPassword(
  credentials: EmailPasswordCredentials,
): Promise<AuthResult> {
  const configured = ensureConfigured();
  if (configured) return configured;

  const fieldErrors = validateLoginForm(credentials);
  if (fieldErrors?.email) return fail(fieldErrors.email);
  if (fieldErrors?.password) return fail(fieldErrors.password);

  const { data, error } = await supabase.auth.signInWithPassword({
    email: credentials.email.trim(),
    password: credentials.password,
  });

  if (error) {
    return fail(mapSupabaseAuthError(error), error.message, error.message);
  }

  // Unconfirmed accounts can return a user without a session.
  if (!data.session) {
    return fail(
      data.user ? "email_not_confirmed" : "unknown",
      data.user ? "email_not_confirmed" : "no_session",
    );
  }

  await ensureProfile({
    userId: data.session.user.id,
    displayName:
      typeof data.session.user.user_metadata?.display_name === "string"
        ? data.session.user.user_metadata.display_name
        : data.session.user.email?.split("@")[0] ?? "ScoutIQ",
  });

  return { ok: true, session: await buildAuthSession(data.session) };
}

export async function signUp(
  credentials: SignUpCredentials,
): Promise<AuthResult> {
  const configured = ensureConfigured();
  if (configured) return configured;

  const fieldErrors = validateRegisterForm(credentials);
  if (fieldErrors?.displayName) return fail(fieldErrors.displayName);
  if (fieldErrors?.email) return fail(fieldErrors.email);
  if (fieldErrors?.password) return fail(fieldErrors.password);
  if (fieldErrors?.confirmPassword) return fail(fieldErrors.confirmPassword);

  const displayName = credentials.displayName.trim();
  const { data, error } = await supabase.auth.signUp({
    email: credentials.email.trim(),
    password: credentials.password,
    options: {
      data: {
        display_name: displayName,
      },
      emailRedirectTo: getSupabaseRedirectUrl(),
    },
  });

  if (error) {
    return fail(mapSupabaseAuthError(error));
  }

  if (!data.session) {
    // Email confirmation may be required — account created, no active session yet.
    return { ok: true, session: null };
  }

  await ensureProfile({
    userId: data.session.user.id,
    displayName,
    language: "de",
    theme: readStoredThemePreference(),
    plan: "free",
  });

  return { ok: true, session: await buildAuthSession(data.session) };
}

export async function resetPassword(email: string): Promise<AuthResult> {
  const configured = ensureConfigured();
  if (configured) return configured;

  const emailError = validateEmail(email);
  if (emailError) return fail(emailError);

  const { error } = await supabase.auth.resetPasswordForEmail(email.trim(), {
    redirectTo: getSupabaseRedirectUrl(),
  });

  if (error) {
    return fail(mapSupabaseAuthError(error));
  }

  // Always succeed from the UI to avoid email enumeration.
  return { ok: true, session: null };
}

export async function updateProfile(
  input: ProfileUpdateInput,
): Promise<AuthResult> {
  const configured = ensureConfigured();
  if (configured) return configured;

  const { data: sessionData, error: sessionError } =
    await supabase.auth.getSession();

  if (sessionError || !sessionData.session) {
    return fail("invalid_credentials");
  }

  const userId = sessionData.session.user.id;

  if (input.displayName) {
    const { error: metaError } = await supabase.auth.updateUser({
      data: { display_name: input.displayName.trim() },
    });
    if (metaError) {
      return fail(mapSupabaseAuthError(metaError));
    }
  }

  await ensureProfile({
    userId,
    displayName:
      input.displayName?.trim() ||
      (typeof sessionData.session.user.user_metadata?.display_name === "string"
        ? sessionData.session.user.user_metadata.display_name
        : sessionData.session.user.email?.split("@")[0] ?? "ScoutIQ"),
  });

  // Profile table may not be created yet — auth metadata still updates.
  await updateProfileRow(userId, {
    displayName: input.displayName,
    language: input.language,
    plan: input.plan,
  });

  const { data: refreshed, error: refreshError } =
    await supabase.auth.getSession();
  if (refreshError || !refreshed.session) {
    return fail("unknown");
  }

  return { ok: true, session: await buildAuthSession(refreshed.session) };
}

export async function signOut(): Promise<void> {
  if (!hasSupabaseConfig()) return;
  await supabase.auth.signOut();
}

/** Google/Apple are intentionally not implemented yet. */
export async function signInWithOAuth(): Promise<AuthResult> {
  return fail("oauth_unavailable");
}
