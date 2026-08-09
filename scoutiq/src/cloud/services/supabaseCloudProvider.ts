import { hasSupabaseConfig, supabase } from "../../lib/supabase";
import { nowIso, resolveByUpdatedAt } from "../conflict";
import { localCloudStore } from "../local/localCloudStore";
import {
  analysisFromRow,
  analysisToRow,
  defaultProfile,
  defaultSettings,
  profileFromRow,
  profileToRow,
  saveMetadataFromRow,
  saveMetadataToRow,
  settingsFromRow,
  settingsToRow,
  shortlistFromRow,
  shortlistToRow,
  type AnalysisHistoryRow,
  type AppSettingsRow,
  type ProfileRow,
  type SaveMetadataRow,
  type TransferShortlistRow,
} from "../mappers";
import type {
  AnalysisHistoryItem,
  AppSettingsSyncData,
  DomainSyncResult,
  SaveMetadataSyncData,
  SyncConflict,
  SyncStatus,
  TransferShortlistItem,
  UserProfileSyncData,
} from "../types";
import { syncCollectionByUpdatedAt } from "./syncHelpers";
import type {
  AnalysisHistorySyncService,
  CloudSyncProvider,
  ProfileSyncService,
  SaveMetadataSyncService,
  SettingsSyncService,
  TransferShortlistSyncService,
} from "./types";

interface AuthContext {
  userId: string;
  email: string;
  displayName?: string;
}

async function requireAuth(): Promise<AuthContext | null> {
  if (!hasSupabaseConfig()) return null;

  const { data, error } = await supabase.auth.getUser();
  if (error || !data.user) return null;

  const displayName =
    typeof data.user.user_metadata?.display_name === "string"
      ? data.user.user_metadata.display_name
      : undefined;

  return {
    userId: data.user.id,
    email: data.user.email ?? "",
    displayName,
  };
}

function domainError(
  domain: DomainSyncResult["domain"],
  status: SyncStatus,
  messageKey: string,
): DomainSyncResult {
  return {
    domain,
    status,
    pushed: 0,
    pulled: 0,
    conflicts: 0,
    messageKey,
  };
}

/** Map PostgREST / Postgres errors to a clearer i18n key. */
function messageKeyForSupabaseError(error: {
  code?: string;
  message?: string;
}): string {
  const code = error.code ?? "";
  const message = (error.message ?? "").toLowerCase();

  if (
    code === "PGRST205" ||
    code === "42P01" ||
    message.includes("could not find the table") ||
    message.includes("schema cache") ||
    (message.includes("relation") && message.includes("does not exist"))
  ) {
    return "cloud.status.schemaMissing";
  }

  if (
    code === "42501" ||
    message.includes("permission denied") ||
    message.includes("row-level security") ||
    message.includes("rls")
  ) {
    return "cloud.status.rlsDenied";
  }

  return "cloud.status.error";
}

function domainErrorFromSupabase(
  domain: DomainSyncResult["domain"],
  error: { code?: string; message?: string },
): DomainSyncResult {
  return domainError(domain, "error", messageKeyForSupabaseError(error));
}

function errorFromUnknown(error: unknown): { code?: string; message?: string } {
  if (error && typeof error === "object") {
    const record = error as { code?: unknown; message?: unknown };
    return {
      code: typeof record.code === "string" ? record.code : undefined,
      message:
        typeof record.message === "string"
          ? record.message
          : String(error),
    };
  }
  return { message: String(error) };
}

async function safeDomainSync(
  domain: DomainSyncResult["domain"],
  run: () => Promise<{
    result: DomainSyncResult;
    conflicts: SyncConflict<
      | UserProfileSyncData
      | AppSettingsSyncData
      | SaveMetadataSyncData
      | AnalysisHistoryItem
      | TransferShortlistItem
    >[];
  }>,
): Promise<{
  result: DomainSyncResult;
  conflicts: SyncConflict<
    | UserProfileSyncData
    | AppSettingsSyncData
    | SaveMetadataSyncData
    | AnalysisHistoryItem
    | TransferShortlistItem
  >[];
}> {
  try {
    return await run();
  } catch (error) {
    console.warn(`[cloud] ${domain} sync failed:`, error);
    return {
      conflicts: [],
      result: domainErrorFromSupabase(domain, errorFromUnknown(error)),
    };
  }
}

async function syncProfileDomain(
  auth: AuthContext,
): Promise<{ result: DomainSyncResult; conflicts: SyncConflict<UserProfileSyncData>[] }> {
  const local =
    localCloudStore.getProfile(auth.userId) ??
    defaultProfile(auth.userId, auth.email, auth.displayName);

  const { data, error } = await supabase
    .from("profiles")
    .select(
      "user_id, display_name, language, theme, plan, created_at, updated_at",
    )
    .eq("user_id", auth.userId)
    .maybeSingle();

  if (error) {
    console.warn("[cloud] profile pull failed:", error.message);
    return {
      conflicts: [],
      result: domainErrorFromSupabase("profile", error),
    };
  }

  const remote = data
    ? profileFromRow(data as ProfileRow, auth.email)
    : null;

  let winner: UserProfileSyncData;
  let pushed = 0;
  let pulled = 0;
  const conflicts: SyncConflict<UserProfileSyncData>[] = [];

  if (!remote) {
    winner = { ...local, userId: auth.userId, email: auth.email };
    const { error: upsertError } = await supabase
      .from("profiles")
      .upsert(
        {
          ...profileToRow(winner),
          created_at: nowIso(),
        },
        { onConflict: "user_id" },
      );
    if (upsertError) {
      console.warn("[cloud] profile push failed:", upsertError.message);
      return {
        conflicts: [],
        result: domainErrorFromSupabase("profile", upsertError),
      };
    }
    pushed = 1;
  } else if (local.updatedAt === remote.updatedAt) {
    winner = remote;
  } else {
    if (local.updatedAt !== remote.updatedAt) {
      conflicts.push({
        domain: "profile",
        entityId: auth.userId,
        local,
        remote,
        suggested:
          remote.updatedAt > local.updatedAt ? "remote_wins" : "local_wins",
      });
    }
    winner = resolveByUpdatedAt(local, remote);
    if (winner === remote) {
      pulled = 1;
    } else {
      const { error: upsertError } = await supabase
        .from("profiles")
        .upsert(profileToRow(winner), { onConflict: "user_id" });
      if (upsertError) {
        console.warn("[cloud] profile push failed:", upsertError.message);
        return {
          conflicts,
          result: domainErrorFromSupabase("profile", upsertError),
        };
      }
      pushed = 1;
    }
  }

  localCloudStore.setProfile(auth.userId, {
    ...winner,
    email: auth.email,
  });

  return {
    conflicts,
    result: {
      domain: "profile",
      pushed,
      pulled,
      conflicts: conflicts.length,
      status: "synced",
      messageKey: "cloud.status.synced",
    },
  };
}

async function syncSettingsDomain(
  auth: AuthContext,
): Promise<{ result: DomainSyncResult; conflicts: SyncConflict<AppSettingsSyncData>[] }> {
  const local =
    localCloudStore.getSettings(auth.userId) ?? defaultSettings(auth.userId);

  const { data, error } = await supabase
    .from("app_settings")
    .select(
      "user_id, theme_preference, language, notifications_enabled, created_at, updated_at",
    )
    .eq("user_id", auth.userId)
    .maybeSingle();

  if (error) {
    console.warn("[cloud] settings pull failed:", error.message);
    return {
      conflicts: [],
      result: domainErrorFromSupabase("settings", error),
    };
  }

  const remote = data ? settingsFromRow(data as AppSettingsRow) : null;
  let winner: AppSettingsSyncData;
  let pushed = 0;
  let pulled = 0;
  const conflicts: SyncConflict<AppSettingsSyncData>[] = [];

  if (!remote) {
    winner = { ...local, userId: auth.userId };
    const { error: upsertError } = await supabase
      .from("app_settings")
      .upsert(
        {
          ...settingsToRow(winner),
          created_at: nowIso(),
        },
        { onConflict: "user_id" },
      );
    if (upsertError) {
      console.warn("[cloud] settings push failed:", upsertError.message);
      return {
        conflicts: [],
        result: domainErrorFromSupabase("settings", upsertError),
      };
    }
    pushed = 1;
  } else if (local.updatedAt === remote.updatedAt) {
    winner = remote;
  } else {
    conflicts.push({
      domain: "settings",
      entityId: auth.userId,
      local,
      remote,
      suggested:
        remote.updatedAt > local.updatedAt ? "remote_wins" : "local_wins",
    });
    winner = resolveByUpdatedAt(local, remote);
    if (winner === remote) {
      pulled = 1;
    } else {
      const { error: upsertError } = await supabase
        .from("app_settings")
        .upsert(settingsToRow(winner), { onConflict: "user_id" });
      if (upsertError) {
        console.warn("[cloud] settings push failed:", upsertError.message);
        return {
          conflicts,
          result: domainErrorFromSupabase("settings", upsertError),
        };
      }
      pushed = 1;
    }
  }

  localCloudStore.setSettings(auth.userId, winner);

  // Keep profiles.theme aligned when settings win locally or remotely.
  await supabase
    .from("profiles")
    .update({
      theme: winner.themePreference,
      updated_at: winner.updatedAt,
    })
    .eq("user_id", auth.userId);

  return {
    conflicts,
    result: {
      domain: "settings",
      pushed,
      pulled,
      conflicts: conflicts.length,
      status: "synced",
      messageKey: "cloud.status.synced",
    },
  };
}

async function syncSaveMetadataDomain(auth: AuthContext) {
  const local = localCloudStore.getSaveMetadata(auth.userId);
  const { data, error } = await supabase
    .from("save_metadata")
    .select(
      "id, user_id, save_name, club_name, competition, season, game_date, last_opened_at, file_name, file_size_bytes, created_at, updated_at, deleted_at",
    )
    .eq("user_id", auth.userId);

  if (error) {
    console.warn("[cloud] save_metadata pull failed:", error.message);
    return {
      conflicts: [] as SyncConflict<SaveMetadataSyncData>[],
      result: domainErrorFromSupabase("saveMetadata", error),
    };
  }

  const remote = ((data ?? []) as SaveMetadataRow[]).map(saveMetadataFromRow);

  return syncCollectionByUpdatedAt({
    domain: "saveMetadata",
    localItems: local,
    remoteItems: remote,
    toRow: saveMetadataToRow,
    upsertRemote: async (rows) => {
      const { error: upsertError } = await supabase
        .from("save_metadata")
        .upsert(rows, { onConflict: "id" });
      if (upsertError) throw upsertError;
    },
    setLocal: (items) => localCloudStore.setSaveMetadata(auth.userId, items),
  });
}

async function syncAnalysisHistoryDomain(auth: AuthContext) {
  const local = localCloudStore.getAnalysisHistory(auth.userId);
  const { data, error } = await supabase
    .from("analysis_history")
    .select(
      "id, user_id, title, category, summary, created_at, updated_at, deleted_at",
    )
    .eq("user_id", auth.userId);

  if (error) {
    console.warn("[cloud] analysis_history pull failed:", error.message);
    return {
      conflicts: [] as SyncConflict<AnalysisHistoryItem>[],
      result: domainErrorFromSupabase("analysisHistory", error),
    };
  }

  const remote = ((data ?? []) as AnalysisHistoryRow[]).map(analysisFromRow);

  return syncCollectionByUpdatedAt({
    domain: "analysisHistory",
    localItems: local,
    remoteItems: remote,
    toRow: analysisToRow,
    upsertRemote: async (rows) => {
      const { error: upsertError } = await supabase
        .from("analysis_history")
        .upsert(rows, { onConflict: "id" });
      if (upsertError) throw upsertError;
    },
    setLocal: (items) => localCloudStore.setAnalysisHistory(auth.userId, items),
  });
}

async function syncTransferShortlistDomain(auth: AuthContext) {
  const local = localCloudStore.getTransferShortlist(auth.userId);
  const { data, error } = await supabase
    .from("transfer_shortlist")
    .select(
      "id, user_id, player_name, position, club, age, estimated_value, priority, notes, created_at, updated_at, deleted_at",
    )
    .eq("user_id", auth.userId);

  if (error) {
    console.warn("[cloud] transfer_shortlist pull failed:", error.message);
    return {
      conflicts: [] as SyncConflict<TransferShortlistItem>[],
      result: domainErrorFromSupabase("transferShortlist", error),
    };
  }

  const remote = ((data ?? []) as TransferShortlistRow[]).map(shortlistFromRow);

  return syncCollectionByUpdatedAt({
    domain: "transferShortlist",
    localItems: local,
    remoteItems: remote,
    toRow: shortlistToRow,
    upsertRemote: async (rows) => {
      const { error: upsertError } = await supabase
        .from("transfer_shortlist")
        .upsert(rows, { onConflict: "id" });
      if (upsertError) throw upsertError;
    },
    setLocal: (items) =>
      localCloudStore.setTransferShortlist(auth.userId, items),
  });
}

/**
 * Real Supabase cloud provider.
 * Supabase is primary; localStorage is an offline cache only.
 */
export function createSupabaseCloudProvider(): CloudSyncProvider {
  const profile: ProfileSyncService = {
    domain: "profile",
    async getLocal() {
      const auth = await requireAuth();
      if (!auth) return null;
      return localCloudStore.getProfile(auth.userId);
    },
    async saveLocal(data) {
      const auth = await requireAuth();
      if (!auth) return;
      localCloudStore.setProfile(auth.userId, {
        ...data,
        userId: auth.userId,
        updatedAt: nowIso(),
      });
    },
    async sync() {
      const auth = await requireAuth();
      if (!auth) {
        return domainError("profile", "unsigned", "cloud.status.unsigned");
      }
      if (!navigator.onLine) {
        return domainError("profile", "offline", "cloud.status.offline");
      }
      return (await syncProfileDomain(auth)).result;
    },
  };

  const settings: SettingsSyncService = {
    domain: "settings",
    async getLocal() {
      const auth = await requireAuth();
      if (!auth) return null;
      return localCloudStore.getSettings(auth.userId);
    },
    async saveLocal(data) {
      const auth = await requireAuth();
      if (!auth) return;
      localCloudStore.setSettings(auth.userId, {
        ...data,
        userId: auth.userId,
        updatedAt: nowIso(),
      });
    },
    async sync() {
      const auth = await requireAuth();
      if (!auth) {
        return domainError("settings", "unsigned", "cloud.status.unsigned");
      }
      if (!navigator.onLine) {
        return domainError("settings", "offline", "cloud.status.offline");
      }
      return (await syncSettingsDomain(auth)).result;
    },
  };

  const saveMetadata: SaveMetadataSyncService = {
    domain: "saveMetadata",
    async listLocal() {
      const auth = await requireAuth();
      if (!auth) return [];
      return localCloudStore.getSaveMetadata(auth.userId);
    },
    async saveLocal(items) {
      const auth = await requireAuth();
      if (!auth) return;
      localCloudStore.setSaveMetadata(
        auth.userId,
        items.map((item) => ({
          ...item,
          userId: auth.userId,
          updatedAt: item.updatedAt || nowIso(),
        })),
      );
    },
    async sync() {
      const auth = await requireAuth();
      if (!auth) {
        return domainError("saveMetadata", "unsigned", "cloud.status.unsigned");
      }
      if (!navigator.onLine) {
        return domainError("saveMetadata", "offline", "cloud.status.offline");
      }
      try {
        return (await syncSaveMetadataDomain(auth)).result;
      } catch (error) {
        console.warn("[cloud] save_metadata sync failed:", error);
        return domainErrorFromSupabase(
          "saveMetadata",
          errorFromUnknown(error),
        );
      }
    },
  };

  const analysisHistory: AnalysisHistorySyncService = {
    domain: "analysisHistory",
    async listLocal() {
      const auth = await requireAuth();
      if (!auth) return [];
      return localCloudStore.getAnalysisHistory(auth.userId);
    },
    async saveLocal(items) {
      const auth = await requireAuth();
      if (!auth) return;
      localCloudStore.setAnalysisHistory(
        auth.userId,
        items.map((item) => ({
          ...item,
          userId: auth.userId,
          updatedAt: item.updatedAt || nowIso(),
        })),
      );
    },
    async sync() {
      const auth = await requireAuth();
      if (!auth) {
        return domainError(
          "analysisHistory",
          "unsigned",
          "cloud.status.unsigned",
        );
      }
      if (!navigator.onLine) {
        return domainError(
          "analysisHistory",
          "offline",
          "cloud.status.offline",
        );
      }
      try {
        return (await syncAnalysisHistoryDomain(auth)).result;
      } catch (error) {
        console.warn("[cloud] analysis_history sync failed:", error);
        return domainErrorFromSupabase(
          "analysisHistory",
          errorFromUnknown(error),
        );
      }
    },
  };

  const transferShortlist: TransferShortlistSyncService = {
    domain: "transferShortlist",
    async listLocal() {
      const auth = await requireAuth();
      if (!auth) return [];
      return localCloudStore.getTransferShortlist(auth.userId);
    },
    async saveLocal(items) {
      const auth = await requireAuth();
      if (!auth) return;
      localCloudStore.setTransferShortlist(
        auth.userId,
        items.map((item) => ({
          ...item,
          userId: auth.userId,
          updatedAt: item.updatedAt || nowIso(),
        })),
      );
    },
    async sync() {
      const auth = await requireAuth();
      if (!auth) {
        return domainError(
          "transferShortlist",
          "unsigned",
          "cloud.status.unsigned",
        );
      }
      if (!navigator.onLine) {
        return domainError(
          "transferShortlist",
          "offline",
          "cloud.status.offline",
        );
      }
      try {
        return (await syncTransferShortlistDomain(auth)).result;
      } catch (error) {
        console.warn("[cloud] transfer_shortlist sync failed:", error);
        return domainErrorFromSupabase(
          "transferShortlist",
          errorFromUnknown(error),
        );
      }
    },
  };

  return {
    id: "supabase",
    isOnline: () =>
      typeof navigator === "undefined" ? true : navigator.onLine,
    profile,
    settings,
    saveMetadata,
    analysisHistory,
    transferShortlist,
    async syncAll() {
      const auth = await requireAuth();
      if (!auth) {
        const domains = [
          domainError("profile", "unsigned", "cloud.status.unsigned"),
          domainError("settings", "unsigned", "cloud.status.unsigned"),
          domainError("saveMetadata", "unsigned", "cloud.status.unsigned"),
          domainError("analysisHistory", "unsigned", "cloud.status.unsigned"),
          domainError(
            "transferShortlist",
            "unsigned",
            "cloud.status.unsigned",
          ),
        ];
        return { domains, conflicts: [] };
      }

      if (!navigator.onLine) {
        const domains = [
          domainError("profile", "offline", "cloud.status.offline"),
          domainError("settings", "offline", "cloud.status.offline"),
          domainError("saveMetadata", "offline", "cloud.status.offline"),
          domainError("analysisHistory", "offline", "cloud.status.offline"),
          domainError("transferShortlist", "offline", "cloud.status.offline"),
        ];
        return { domains, conflicts: [] };
      }

      try {
        const [
          profileSync,
          settingsSync,
          saveSync,
          analysisSync,
          shortlistSync,
        ] = await Promise.all([
          safeDomainSync("profile", () => syncProfileDomain(auth)),
          safeDomainSync("settings", () => syncSettingsDomain(auth)),
          safeDomainSync("saveMetadata", () => syncSaveMetadataDomain(auth)),
          safeDomainSync("analysisHistory", () =>
            syncAnalysisHistoryDomain(auth),
          ),
          safeDomainSync("transferShortlist", () =>
            syncTransferShortlistDomain(auth),
          ),
        ]);

        return {
          domains: [
            profileSync.result,
            settingsSync.result,
            saveSync.result,
            analysisSync.result,
            shortlistSync.result,
          ],
          conflicts: [
            ...profileSync.conflicts,
            ...settingsSync.conflicts,
            ...saveSync.conflicts,
            ...analysisSync.conflicts,
            ...shortlistSync.conflicts,
          ],
        };
      } catch (error) {
        console.warn("[cloud] syncAll failed:", error);
        const mapped = errorFromUnknown(error);
        return {
          domains: [
            domainErrorFromSupabase("profile", mapped),
            domainErrorFromSupabase("settings", mapped),
            domainErrorFromSupabase("saveMetadata", mapped),
            domainErrorFromSupabase("analysisHistory", mapped),
            domainErrorFromSupabase("transferShortlist", mapped),
          ],
          conflicts: [],
        };
      }
    },
  };
}
