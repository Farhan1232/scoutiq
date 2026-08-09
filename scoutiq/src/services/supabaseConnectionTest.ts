import { hasSupabaseConfig, supabase } from "../lib/supabase";
import { getProfile } from "./profileService";

export type ConnectionCheckId = "config" | "auth" | "session" | "profile";

export type ConnectionCheckStatus = "ok" | "warn" | "error" | "skipped";

export interface ConnectionCheck {
  id: ConnectionCheckId;
  status: ConnectionCheckStatus;
  /** i18n key under cloud.connection.checks.* */
  detailKey: string;
  detailParams?: Record<string, string>;
}

export interface ConnectionTestResult {
  ok: boolean;
  checkedAt: string;
  checks: ConnectionCheck[];
}

/**
 * Lightweight Supabase smoke test:
 * 1) Env config present
 * 2) Auth API reachable (getUser)
 * 3) Local/persisted session status
 * 4) Profile row readable when signed in
 */
export async function testSupabaseConnection(): Promise<ConnectionTestResult> {
  const checks: ConnectionCheck[] = [];
  const checkedAt = new Date().toISOString();

  if (!hasSupabaseConfig()) {
    checks.push({
      id: "config",
      status: "error",
      detailKey: "cloud.connection.checks.configMissing",
    });
    return { ok: false, checkedAt, checks };
  }

  checks.push({
    id: "config",
    status: "ok",
    detailKey: "cloud.connection.checks.configOk",
  });

  try {
    const { data: userData, error: userError } = await supabase.auth.getUser();

    if (userError) {
      checks.push({
        id: "auth",
        status: "error",
        detailKey: "cloud.connection.checks.authError",
        detailParams: { detail: userError.message },
      });
      checks.push({
        id: "session",
        status: "skipped",
        detailKey: "cloud.connection.checks.skipped",
      });
      checks.push({
        id: "profile",
        status: "skipped",
        detailKey: "cloud.connection.checks.skipped",
      });
      return { ok: false, checkedAt, checks };
    }

    checks.push({
      id: "auth",
      status: "ok",
      detailKey: "cloud.connection.checks.authOk",
    });

    const { data: sessionData, error: sessionError } =
      await supabase.auth.getSession();

    if (sessionError) {
      checks.push({
        id: "session",
        status: "error",
        detailKey: "cloud.connection.checks.sessionError",
        detailParams: { detail: sessionError.message },
      });
      checks.push({
        id: "profile",
        status: "skipped",
        detailKey: "cloud.connection.checks.skipped",
      });
      return { ok: false, checkedAt, checks };
    }

    const session = sessionData.session;
    const user = userData.user ?? session?.user ?? null;

    if (!session || !user) {
      checks.push({
        id: "session",
        status: "warn",
        detailKey: "cloud.connection.checks.sessionSignedOut",
      });
      checks.push({
        id: "profile",
        status: "skipped",
        detailKey: "cloud.connection.checks.profileNeedsSignIn",
      });
      return { ok: true, checkedAt, checks };
    }

    checks.push({
      id: "session",
      status: "ok",
      detailKey: "cloud.connection.checks.sessionOk",
      detailParams: { email: user.email ?? user.id },
    });

    const profile = await getProfile(user.id);
    if (!profile) {
      checks.push({
        id: "profile",
        status: "warn",
        detailKey: "cloud.connection.checks.profileMissing",
      });
      return { ok: true, checkedAt, checks };
    }

    checks.push({
      id: "profile",
      status: "ok",
      detailKey: "cloud.connection.checks.profileOk",
      detailParams: { name: profile.display_name },
    });

    return { ok: true, checkedAt, checks };
  } catch (error) {
    const detail =
      error instanceof Error ? error.message : "Unbekannter Fehler";
    checks.push({
      id: "auth",
      status: "error",
      detailKey: "cloud.connection.checks.networkError",
      detailParams: { detail },
    });
    return { ok: false, checkedAt, checks };
  }
}
