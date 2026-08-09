import type {
  AnalysisHistoryItem,
  AppSettingsSyncData,
  SaveMetadataSyncData,
  TransferShortlistItem,
  UserProfileSyncData,
} from "../types";
import { localKey, readLocalJson, removeLocalKey, writeLocalJson } from "./storage";

/**
 * Offline cache only — never stores credentials.
 * Keys are scoped per authenticated user id.
 */
function keysFor(userId: string) {
  return {
    profile: localKey([userId, "profile"]),
    settings: localKey([userId, "settings"]),
    saveMetadata: localKey([userId, "saveMetadata"]),
    analysisHistory: localKey([userId, "analysisHistory"]),
    transferShortlist: localKey([userId, "transferShortlist"]),
    lastSyncedAt: localKey([userId, "lastSyncedAt"]),
  };
}

/** Legacy unscoped keys from the mock era — cleared once. */
const LEGACY_KEYS = [
  localKey(["profile"]),
  localKey(["settings"]),
  localKey(["saveMetadata"]),
  localKey(["analysisHistory"]),
  localKey(["transferShortlist"]),
  localKey(["lastSyncedAt"]),
];

let legacyCleared = false;

function clearLegacyMockCache() {
  if (legacyCleared || typeof window === "undefined") return;
  for (const key of LEGACY_KEYS) {
    removeLocalKey(key);
  }
  legacyCleared = true;
}

export const localCloudStore = {
  clearLegacyMockCache,

  getProfile(userId: string): UserProfileSyncData | null {
    clearLegacyMockCache();
    return readLocalJson<UserProfileSyncData | null>(
      keysFor(userId).profile,
      null,
    );
  },

  setProfile(userId: string, profile: UserProfileSyncData) {
    writeLocalJson(keysFor(userId).profile, profile);
  },

  getSettings(userId: string): AppSettingsSyncData | null {
    clearLegacyMockCache();
    return readLocalJson<AppSettingsSyncData | null>(
      keysFor(userId).settings,
      null,
    );
  },

  setSettings(userId: string, settings: AppSettingsSyncData) {
    writeLocalJson(keysFor(userId).settings, settings);
  },

  getSaveMetadata(userId: string): SaveMetadataSyncData[] {
    clearLegacyMockCache();
    return readLocalJson<SaveMetadataSyncData[]>(
      keysFor(userId).saveMetadata,
      [],
    );
  },

  setSaveMetadata(userId: string, items: SaveMetadataSyncData[]) {
    writeLocalJson(keysFor(userId).saveMetadata, items);
  },

  getAnalysisHistory(userId: string): AnalysisHistoryItem[] {
    clearLegacyMockCache();
    return readLocalJson<AnalysisHistoryItem[]>(
      keysFor(userId).analysisHistory,
      [],
    );
  },

  setAnalysisHistory(userId: string, items: AnalysisHistoryItem[]) {
    writeLocalJson(keysFor(userId).analysisHistory, items);
  },

  getTransferShortlist(userId: string): TransferShortlistItem[] {
    clearLegacyMockCache();
    return readLocalJson<TransferShortlistItem[]>(
      keysFor(userId).transferShortlist,
      [],
    );
  },

  setTransferShortlist(userId: string, items: TransferShortlistItem[]) {
    writeLocalJson(keysFor(userId).transferShortlist, items);
  },

  getLastSyncedAt(userId: string): string | null {
    return readLocalJson<string | null>(keysFor(userId).lastSyncedAt, null);
  },

  setLastSyncedAt(userId: string, value: string | null) {
    writeLocalJson(keysFor(userId).lastSyncedAt, value);
  },
};
