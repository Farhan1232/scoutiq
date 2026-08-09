import { supabase } from "../lib/supabase";

export type AuthCallbackKind =
  | "none"
  | "email_confirmed"
  | "password_recovery"
  | "error";

export interface AuthCallbackResult {
  kind: AuthCallbackKind;
  messageKey: string;
  detail?: string;
}

function hasAuthCallbackParams(): boolean {
  if (typeof window === "undefined") return false;

  const url = new URL(window.location.href);
  const hash = new URLSearchParams(url.hash.replace(/^#/, ""));

  return Boolean(
    url.searchParams.get("code") ||
      url.searchParams.get("error") ||
      url.searchParams.get("error_description") ||
      hash.get("access_token") ||
      hash.get("error") ||
      hash.get("error_description") ||
      hash.get("type"),
  );
}

export function isAuthCallbackUrl(): boolean {
  return hasAuthCallbackParams();
}

/**
 * Completes Supabase email-confirm / recovery redirects opened in the browser.
 * Safe to call when no callback params are present (returns kind: none).
 */
export async function handleAuthCallback(): Promise<AuthCallbackResult> {
  if (!hasAuthCallbackParams()) {
    return { kind: "none", messageKey: "auth.callback.none" };
  }

  const url = new URL(window.location.href);
  const hash = new URLSearchParams(url.hash.replace(/^#/, ""));

  const errorCode =
    url.searchParams.get("error_code") || hash.get("error_code");
  const error =
    url.searchParams.get("error_description") ||
    url.searchParams.get("error") ||
    hash.get("error_description") ||
    hash.get("error");

  if (error || errorCode) {
    const expired =
      errorCode === "otp_expired" ||
      (error ?? "").toLowerCase().includes("expired") ||
      (error ?? "").toLowerCase().includes("invalid");

    // Clean error params from the address bar
    window.history.replaceState({}, document.title, url.origin + url.pathname);

    return {
      kind: "error",
      messageKey: expired
        ? "auth.callback.linkExpired"
        : "auth.callback.error",
      detail: expired ? undefined : (error ?? errorCode ?? undefined),
    };
  }

  const type = hash.get("type") ?? url.searchParams.get("type");

  try {
    const code = url.searchParams.get("code");
    if (code) {
      const { error: exchangeError } =
        await supabase.auth.exchangeCodeForSession(code);
      if (exchangeError) {
        return {
          kind: "error",
          messageKey: "auth.callback.error",
          detail: exchangeError.message,
        };
      }
    } else {
      // detectSessionInUrl / getSession picks up implicit hash tokens
      const { error: sessionError } = await supabase.auth.getSession();
      if (sessionError) {
        return {
          kind: "error",
          messageKey: "auth.callback.error",
          detail: sessionError.message,
        };
      }
    }

    // Clean sensitive tokens from the address bar
    window.history.replaceState({}, document.title, url.origin + url.pathname);

    if (type === "recovery") {
      return {
        kind: "password_recovery",
        messageKey: "auth.callback.passwordRecovery",
      };
    }

    return {
      kind: "email_confirmed",
      messageKey: "auth.callback.emailConfirmed",
    };
  } catch (caught) {
    return {
      kind: "error",
      messageKey: "auth.callback.error",
      detail: caught instanceof Error ? caught.message : String(caught),
    };
  }
}
