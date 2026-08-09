import { createClient } from "@supabase/supabase-js";

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL;
const supabaseKey = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY;

const configured = Boolean(supabaseUrl?.trim() && supabaseKey?.trim());

/**
 * `createClient` throws `supabaseUrl is required.` on an empty string, and this
 * module is imported at startup by the auth and cloud-sync code. On a machine
 * with no `.env` that threw during module evaluation and took the entire app
 * down with a blank window — including save loading and the league parser,
 * neither of which needs an account.
 *
 * So an unconfigured build gets a syntactically valid placeholder instead, and
 * `hasSupabaseConfig()` gates every call that would actually reach the network.
 * Nothing is sent anywhere: no request is made unless real credentials exist.
 */
const PLACEHOLDER_URL = "http://localhost:54321";
const PLACEHOLDER_KEY = "unconfigured-local-placeholder";

export const supabase = createClient(
  configured ? supabaseUrl! : PLACEHOLDER_URL,
  configured ? supabaseKey! : PLACEHOLDER_KEY,
  {
    auth: {
      persistSession: true,
      autoRefreshToken: true,
      detectSessionInUrl: true,
      storage: typeof window !== "undefined" ? window.localStorage : undefined,
    },
  },
);

/** True only when a real Supabase project is configured via `.env`. */
export function hasSupabaseConfig(): boolean {
  return configured;
}

export function getSupabaseRedirectUrl(): string {
  const configured = import.meta.env.VITE_SUPABASE_REDIRECT_URL?.trim();
  if (configured) return configured;
  if (typeof window !== "undefined" && window.location?.origin) {
    return window.location.origin;
  }
  return "http://localhost:1420";
}
