import type { SyncDomain } from "../config";
import type {
  AnalysisHistoryItem,
  AppSettingsSyncData,
  DomainSyncResult,
  SaveMetadataSyncData,
  SyncConflict,
  TransferShortlistItem,
  UserProfileSyncData,
} from "../types";

export interface ProfileSyncService {
  readonly domain: "profile";
  getLocal(): Promise<UserProfileSyncData | null>;
  saveLocal(data: UserProfileSyncData): Promise<void>;
  sync(): Promise<DomainSyncResult>;
}

export interface SettingsSyncService {
  readonly domain: "settings";
  getLocal(): Promise<AppSettingsSyncData | null>;
  saveLocal(data: AppSettingsSyncData): Promise<void>;
  sync(): Promise<DomainSyncResult>;
}

export interface SaveMetadataSyncService {
  readonly domain: "saveMetadata";
  listLocal(): Promise<SaveMetadataSyncData[]>;
  saveLocal(items: SaveMetadataSyncData[]): Promise<void>;
  sync(): Promise<DomainSyncResult>;
}

export interface AnalysisHistorySyncService {
  readonly domain: "analysisHistory";
  listLocal(): Promise<AnalysisHistoryItem[]>;
  saveLocal(items: AnalysisHistoryItem[]): Promise<void>;
  sync(): Promise<DomainSyncResult>;
}

export interface TransferShortlistSyncService {
  readonly domain: "transferShortlist";
  listLocal(): Promise<TransferShortlistItem[]>;
  saveLocal(items: TransferShortlistItem[]): Promise<void>;
  sync(): Promise<DomainSyncResult>;
}

export interface CloudSyncProvider {
  readonly id: "local" | "supabase";
  isOnline(): boolean;
  profile: ProfileSyncService;
  settings: SettingsSyncService;
  saveMetadata: SaveMetadataSyncService;
  analysisHistory: AnalysisHistorySyncService;
  transferShortlist: TransferShortlistSyncService;
  syncAll(): Promise<{
    domains: DomainSyncResult[];
    conflicts: SyncConflict<
      | UserProfileSyncData
      | AppSettingsSyncData
      | SaveMetadataSyncData
      | AnalysisHistoryItem
      | TransferShortlistItem
    >[];
  }>;
}

export type DomainService =
  | ProfileSyncService
  | SettingsSyncService
  | SaveMetadataSyncService
  | AnalysisHistorySyncService
  | TransferShortlistSyncService;

export function domainMessage(
  domain: SyncDomain,
  status: DomainSyncResult["status"],
): string {
  return `cloud.domains.${domain}.${status}`;
}
