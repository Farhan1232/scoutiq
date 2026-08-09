/**
 * Authentication configuration for ScoutIQ (Supabase).
 * Google/Apple stay prepared in the UI but are not enabled yet.
 */

export const AUTH_METHODS = {
  emailPassword: true,
  google: false,
  apple: false,
} as const;

/** Shown in the auth UI as coming-soon providers. */
export const FUTURE_AUTH_METHODS = ["google", "apple"] as const;

export type AuthMethodId = keyof typeof AUTH_METHODS;

export function isAuthMethodEnabled(method: AuthMethodId): boolean {
  return AUTH_METHODS[method];
}
