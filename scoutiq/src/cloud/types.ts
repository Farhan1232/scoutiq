import type { SyncDomain } from "./config";

export type SyncStatus =
  | "synced"
  | "syncing"
  | "offline"
  | "error"
  | "unsigned";

export type CloudModeBadge = "cloud" | "offline" | "local";

export type ConflictStrategy = "remote_wins" | "local_wins" | "manual";

/** Base shape for every syncable local/cloud record. */
export interface SyncableEntity {
  id: string;
  userId: string;
  updatedAt: string;
  /** Optional soft-delete marker for cloud tombstones. */
  deletedAt?: string | null;
}

export interface UserProfileSyncData extends SyncableEntity {
  displayName: string;
  email: string;
  language: "de" | "en";
  plan: "free" | "pro" | "elite";
  initials: string;
  theme?: "dark" | "light" | "system";
}

export interface AppSettingsSyncData extends SyncableEntity {
  themePreference: "dark" | "light" | "system";
  language: "de" | "en";
  notificationsEnabled: boolean;
}

/**
 * Metadata only — never the full Football Manager save blob.
 * Full `.fm` / save file upload is explicitly out of scope for now.
 */
export interface SaveMetadataSyncData extends SyncableEntity {
  saveName: string;
  clubName: string;
  competition: string;
  season: string;
  gameDate: string;
  lastOpenedAt: string;
  fileName: string;
  fileSizeBytes?: number;
}

export interface AnalysisHistoryItem extends SyncableEntity {
  title: string;
  category: "tactics" | "transfer" | "opponent" | "training" | "other";
  summary: string;
  createdAt: string;
}

export interface TransferShortlistItem extends SyncableEntity {
  playerName: string;
  position: string;
  club: string;
  age: number;
  estimatedValue: string;
  priority: "high" | "medium" | "low";
  notes?: string;
}

export interface SyncConflict<T extends SyncableEntity> {
  domain: SyncDomain;
  entityId: string;
  local: T;
  remote: T;
  /** Suggested resolution based on updated_at (newer wins). */
  suggested: ConflictStrategy;
}

export interface DomainSyncResult {
  domain: SyncDomain;
  pushed: number;
  pulled: number;
  conflicts: number;
  status: SyncStatus;
  messageKey: string;
  messageParams?: Record<string, string>;
}

export interface SyncRunResult {
  status: SyncStatus;
  startedAt: string;
  finishedAt: string;
  domains: DomainSyncResult[];
  messageKey: string;
}

export interface CloudSyncSnapshot {
  profile: UserProfileSyncData | null;
  settings: AppSettingsSyncData | null;
  saveMetadata: SaveMetadataSyncData[];
  analysisHistory: AnalysisHistoryItem[];
  transferShortlist: TransferShortlistItem[];
}
