import { CLOUD_BACKEND, isCloudSyncEnabled } from "./config";
import { nowIso } from "./conflict";
import { localCloudStore } from "./local/localCloudStore";
import { hasSupabaseConfig, supabase } from "../lib/supabase";
import { createLocalCloudProvider } from "./services/localCloudProvider";
import { createSupabaseCloudProvider } from "./services/supabaseCloudProvider";
import type { CloudSyncProvider } from "./services/types";
import type { CloudModeBadge, SyncRunResult, SyncStatus } from "./types";

let provider: CloudSyncProvider | null = null;

export function getCloudProvider(): CloudSyncProvider {
  if (!provider) {
    provider =
      CLOUD_BACKEND === "supabase"
        ? createSupabaseCloudProvider()
        : createLocalCloudProvider();
  }
  return provider;
}

/** Reset cached provider — useful when flipping backend in tests/future toggles. */
export function resetCloudProvider() {
  provider = null;
}

function aggregateStatus(statuses: SyncStatus[]): SyncStatus {
  if (statuses.includes("unsigned")) return "unsigned";
  if (statuses.includes("error")) return "error";
  if (statuses.includes("offline")) return "offline";
  if (statuses.includes("syncing")) return "syncing";
  return "synced";
}

async function currentUserId(): Promise<string | null> {
  if (!hasSupabaseConfig()) return null;
  const { data } = await supabase.auth.getUser();
  return data.user?.id ?? null;
}

export async function runCloudSync(): Promise<SyncRunResult> {
  localCloudStore.clearLegacyMockCache();
  const startedAt = nowIso();
  const active = getCloudProvider();
  const userId = await currentUserId();

  if (!userId) {
    return {
      status: "unsigned",
      startedAt,
      finishedAt: nowIso(),
      domains: [],
      messageKey: "cloud.status.unsigned",
    };
  }

  if (!active.isOnline()) {
    return {
      status: "offline",
      startedAt,
      finishedAt: nowIso(),
      domains: [],
      messageKey: "cloud.status.offline",
    };
  }

  const { domains } = await active.syncAll();
  const status = aggregateStatus(domains.map((item) => item.status));
  const finishedAt = nowIso();

  if (status === "synced") {
    localCloudStore.setLastSyncedAt(userId, finishedAt);
  }

  return {
    status,
    startedAt,
    finishedAt,
    domains,
    messageKey:
      status === "synced"
        ? "cloud.status.synced"
        : status === "offline"
          ? "cloud.status.offline"
          : status === "unsigned"
            ? "cloud.status.unsigned"
            : "cloud.status.error",
  };
}

export async function getLastSyncedAt(): Promise<string | null> {
  const userId = await currentUserId();
  if (!userId) return null;
  return localCloudStore.getLastSyncedAt(userId);
}

export function getInitialSyncStatus(): SyncStatus {
  if (typeof navigator !== "undefined" && !navigator.onLine) {
    return "offline";
  }
  return isCloudSyncEnabled() ? "synced" : "unsigned";
}

export function resolveCloudModeBadge(input: {
  isAuthenticated: boolean;
  isOnline: boolean;
}): CloudModeBadge {
  if (!input.isAuthenticated) return "local";
  if (!input.isOnline) return "offline";
  return "cloud";
}
