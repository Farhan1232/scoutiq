import { nowIso } from "../conflict";
import { localCloudStore } from "../local/localCloudStore";
import { supabase } from "../../lib/supabase";
import type { DomainSyncResult, SyncStatus } from "../types";
import type {
  AnalysisHistorySyncService,
  CloudSyncProvider,
  ProfileSyncService,
  SaveMetadataSyncService,
  SettingsSyncService,
  TransferShortlistSyncService,
} from "./types";

async function currentUserId(): Promise<string | null> {
  const { data } = await supabase.auth.getUser();
  return data.user?.id ?? null;
}

function result(
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

/**
 * Offline/local fallback provider.
 * Used only when CLOUD_BACKEND === "local".
 */
export function createLocalCloudProvider(): CloudSyncProvider {
  const profile: ProfileSyncService = {
    domain: "profile",
    async getLocal() {
      const userId = await currentUserId();
      if (!userId) return null;
      return localCloudStore.getProfile(userId);
    },
    async saveLocal(data) {
      const userId = await currentUserId();
      if (!userId) return;
      localCloudStore.setProfile(userId, { ...data, updatedAt: nowIso() });
    },
    async sync() {
      const userId = await currentUserId();
      if (!userId) {
        return result("profile", "unsigned", "cloud.status.unsigned");
      }
      return result("profile", "synced", "cloud.status.localOnly");
    },
  };

  const settings: SettingsSyncService = {
    domain: "settings",
    async getLocal() {
      const userId = await currentUserId();
      if (!userId) return null;
      return localCloudStore.getSettings(userId);
    },
    async saveLocal(data) {
      const userId = await currentUserId();
      if (!userId) return;
      localCloudStore.setSettings(userId, { ...data, updatedAt: nowIso() });
    },
    async sync() {
      const userId = await currentUserId();
      if (!userId) {
        return result("settings", "unsigned", "cloud.status.unsigned");
      }
      return result("settings", "synced", "cloud.status.localOnly");
    },
  };

  const saveMetadata: SaveMetadataSyncService = {
    domain: "saveMetadata",
    async listLocal() {
      const userId = await currentUserId();
      if (!userId) return [];
      return localCloudStore.getSaveMetadata(userId);
    },
    async saveLocal(items) {
      const userId = await currentUserId();
      if (!userId) return;
      localCloudStore.setSaveMetadata(userId, items);
    },
    async sync() {
      const userId = await currentUserId();
      if (!userId) {
        return result("saveMetadata", "unsigned", "cloud.status.unsigned");
      }
      return result("saveMetadata", "synced", "cloud.status.localOnly");
    },
  };

  const analysisHistory: AnalysisHistorySyncService = {
    domain: "analysisHistory",
    async listLocal() {
      const userId = await currentUserId();
      if (!userId) return [];
      return localCloudStore.getAnalysisHistory(userId);
    },
    async saveLocal(items) {
      const userId = await currentUserId();
      if (!userId) return;
      localCloudStore.setAnalysisHistory(userId, items);
    },
    async sync() {
      const userId = await currentUserId();
      if (!userId) {
        return result("analysisHistory", "unsigned", "cloud.status.unsigned");
      }
      return result("analysisHistory", "synced", "cloud.status.localOnly");
    },
  };

  const transferShortlist: TransferShortlistSyncService = {
    domain: "transferShortlist",
    async listLocal() {
      const userId = await currentUserId();
      if (!userId) return [];
      return localCloudStore.getTransferShortlist(userId);
    },
    async saveLocal(items) {
      const userId = await currentUserId();
      if (!userId) return;
      localCloudStore.setTransferShortlist(userId, items);
    },
    async sync() {
      const userId = await currentUserId();
      if (!userId) {
        return result(
          "transferShortlist",
          "unsigned",
          "cloud.status.unsigned",
        );
      }
      return result("transferShortlist", "synced", "cloud.status.localOnly");
    },
  };

  return {
    id: "local",
    isOnline: () =>
      typeof navigator === "undefined" ? true : navigator.onLine,
    profile,
    settings,
    saveMetadata,
    analysisHistory,
    transferShortlist,
    async syncAll() {
      const domains = await Promise.all([
        profile.sync(),
        settings.sync(),
        saveMetadata.sync(),
        analysisHistory.sync(),
        transferShortlist.sync(),
      ]);
      return { domains, conflicts: [] };
    },
  };
}
