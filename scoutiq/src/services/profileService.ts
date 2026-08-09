import { supabase } from "../lib/supabase";
import type { ThemePreference } from "../theme/theme";
import type { UserLanguage, UserPlan } from "../auth/types";

/** Row shape for public.profiles (prepared for Supabase). */
export interface ProfileRow {
  user_id: string;
  display_name: string;
  language: UserLanguage;
  theme: ThemePreference;
  plan: UserPlan;
  created_at: string;
  updated_at: string;
}

export interface ProfileCreateInput {
  userId: string;
  displayName: string;
  language?: UserLanguage;
  theme?: ThemePreference;
  plan?: UserPlan;
}

export interface ProfileUpdateInput {
  displayName?: string;
  language?: UserLanguage;
  theme?: ThemePreference;
  plan?: UserPlan;
}

function nowIso(): string {
  return new Date().toISOString();
}

export async function getProfile(userId: string): Promise<ProfileRow | null> {
  const { data, error } = await supabase
    .from("profiles")
    .select(
      "user_id, display_name, language, theme, plan, created_at, updated_at",
    )
    .eq("user_id", userId)
    .maybeSingle();

  if (error) {
    console.warn("[profileService] getProfile failed:", error.message);
    return null;
  }

  return data as ProfileRow | null;
}

export async function ensureProfile(
  input: ProfileCreateInput,
): Promise<ProfileRow | null> {
  const existing = await getProfile(input.userId);
  if (existing) return existing;

  const timestamp = nowIso();
  const row = {
    user_id: input.userId,
    display_name: input.displayName.trim(),
    language: input.language ?? "de",
    theme: input.theme ?? "dark",
    plan: input.plan ?? "free",
    created_at: timestamp,
    updated_at: timestamp,
  };

  const { data, error } = await supabase
    .from("profiles")
    .upsert(row, { onConflict: "user_id" })
    .select(
      "user_id, display_name, language, theme, plan, created_at, updated_at",
    )
    .maybeSingle();

  if (error) {
    console.warn("[profileService] ensureProfile failed:", error.message);
    return null;
  }

  return data as ProfileRow | null;
}

export async function updateProfileRow(
  userId: string,
  input: ProfileUpdateInput,
): Promise<ProfileRow | null> {
  const patch: Record<string, string> = {
    updated_at: nowIso(),
  };

  if (input.displayName !== undefined) {
    patch.display_name = input.displayName.trim();
  }
  if (input.language !== undefined) {
    patch.language = input.language;
  }
  if (input.theme !== undefined) {
    patch.theme = input.theme;
  }
  if (input.plan !== undefined) {
    patch.plan = input.plan;
  }

  const { data, error } = await supabase
    .from("profiles")
    .update(patch)
    .eq("user_id", userId)
    .select(
      "user_id, display_name, language, theme, plan, created_at, updated_at",
    )
    .maybeSingle();

  if (error) {
    console.warn("[profileService] updateProfileRow failed:", error.message);
    return null;
  }

  return data as ProfileRow | null;
}
