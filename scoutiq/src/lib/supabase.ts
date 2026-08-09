import { createClient } from "@supabase/supabase-js";

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL;
const supabaseKey = import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY;

export const supabase = createClient(supabaseUrl ?? "", supabaseKey ?? "", {
  auth: {
    persistSession: true,
    autoRefreshToken: true,
    detectSessionInUrl: true,
    storage: typeof window !== "undefined" ? window.localStorage : undefined,
  },
});

export function hasSupabaseConfig(): boolean {
  return Boolean(supabaseUrl?.trim() && supabaseKey?.trim());
}

export function getSupabaseRedirectUrl(): string {
  const configured = import.meta.env.VITE_SUPABASE_REDIRECT_URL?.trim();
  if (configured) return configured;
  if (typeof window !== "undefined" && window.location?.origin) {
    return window.location.origin;
  }
  return "http://localhost:1420";
}
