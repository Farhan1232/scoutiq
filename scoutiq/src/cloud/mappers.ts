import { nowIso } from "./conflict";
import type {
  AnalysisHistoryItem,
  AppSettingsSyncData,
  SaveMetadataSyncData,
  TransferShortlistItem,
  UserProfileSyncData,
} from "./types";

function initialsFromName(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  if (parts.length === 0) return "SQ";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return `${parts[0][0]}${parts[1][0]}`.toUpperCase();
}

export interface ProfileRow {
  user_id: string;
  display_name: string;
  language: "de" | "en";
  theme: "dark" | "light" | "system";
  plan: "free" | "pro" | "elite";
  created_at: string;
  updated_at: string;
}

export interface AppSettingsRow {
  user_id: string;
  theme_preference: "dark" | "light" | "system";
  language: "de" | "en";
  notifications_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface SaveMetadataRow {
  id: string;
  user_id: string;
  save_name: string;
  club_name: string;
  competition: string;
  season: string;
  game_date: string;
  last_opened_at: string;
  file_name: string;
  file_size_bytes: number | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface AnalysisHistoryRow {
  id: string;
  user_id: string;
  title: string;
  category: AnalysisHistoryItem["category"];
  summary: string;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface TransferShortlistRow {
  id: string;
  user_id: string;
  player_name: string;
  position: string;
  club: string;
  age: number;
  estimated_value: string;
  priority: TransferShortlistItem["priority"];
  notes: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export function profileFromRow(
  row: ProfileRow,
  email: string,
): UserProfileSyncData {
  return {
    id: row.user_id,
    userId: row.user_id,
    displayName: row.display_name,
    email,
    language: row.language,
    plan: row.plan,
    initials: initialsFromName(row.display_name),
    theme: row.theme,
    updatedAt: row.updated_at,
  };
}

export function profileToRow(profile: UserProfileSyncData): Partial<ProfileRow> {
  return {
    user_id: profile.userId,
    display_name: profile.displayName,
    language: profile.language,
    theme: profile.theme ?? "dark",
    plan: profile.plan,
    updated_at: profile.updatedAt || nowIso(),
  };
}

export function settingsFromRow(row: AppSettingsRow): AppSettingsSyncData {
  return {
    id: row.user_id,
    userId: row.user_id,
    themePreference: row.theme_preference,
    language: row.language,
    notificationsEnabled: row.notifications_enabled,
    updatedAt: row.updated_at,
  };
}

export function settingsToRow(
  settings: AppSettingsSyncData,
): Partial<AppSettingsRow> {
  return {
    user_id: settings.userId,
    theme_preference: settings.themePreference,
    language: settings.language,
    notifications_enabled: settings.notificationsEnabled,
    updated_at: settings.updatedAt || nowIso(),
  };
}

export function saveMetadataFromRow(row: SaveMetadataRow): SaveMetadataSyncData {
  return {
    id: row.id,
    userId: row.user_id,
    saveName: row.save_name,
    clubName: row.club_name,
    competition: row.competition,
    season: row.season,
    gameDate: row.game_date,
    lastOpenedAt: row.last_opened_at,
    fileName: row.file_name,
    fileSizeBytes: row.file_size_bytes ?? undefined,
    updatedAt: row.updated_at,
    deletedAt: row.deleted_at,
  };
}

export function saveMetadataToRow(
  item: SaveMetadataSyncData,
): Partial<SaveMetadataRow> {
  return {
    id: item.id,
    user_id: item.userId,
    save_name: item.saveName,
    club_name: item.clubName,
    competition: item.competition,
    season: item.season,
    game_date: item.gameDate,
    last_opened_at: item.lastOpenedAt,
    file_name: item.fileName,
    file_size_bytes: item.fileSizeBytes ?? null,
    updated_at: item.updatedAt || nowIso(),
    deleted_at: item.deletedAt ?? null,
  };
}

export function analysisFromRow(row: AnalysisHistoryRow): AnalysisHistoryItem {
  return {
    id: row.id,
    userId: row.user_id,
    title: row.title,
    category: row.category,
    summary: row.summary,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    deletedAt: row.deleted_at,
  };
}

export function analysisToRow(
  item: AnalysisHistoryItem,
): Partial<AnalysisHistoryRow> {
  return {
    id: item.id,
    user_id: item.userId,
    title: item.title,
    category: item.category,
    summary: item.summary,
    created_at: item.createdAt,
    updated_at: item.updatedAt || nowIso(),
    deleted_at: item.deletedAt ?? null,
  };
}

export function shortlistFromRow(
  row: TransferShortlistRow,
): TransferShortlistItem {
  return {
    id: row.id,
    userId: row.user_id,
    playerName: row.player_name,
    position: row.position,
    club: row.club,
    age: row.age,
    estimatedValue: row.estimated_value,
    priority: row.priority,
    notes: row.notes ?? undefined,
    updatedAt: row.updated_at,
    deletedAt: row.deleted_at,
  };
}

export function shortlistToRow(
  item: TransferShortlistItem,
): Partial<TransferShortlistRow> {
  return {
    id: item.id,
    user_id: item.userId,
    player_name: item.playerName,
    position: item.position,
    club: item.club,
    age: item.age,
    estimated_value: item.estimatedValue,
    priority: item.priority,
    notes: item.notes ?? null,
    updated_at: item.updatedAt || nowIso(),
    deleted_at: item.deletedAt ?? null,
  };
}

export function defaultProfile(
  userId: string,
  email: string,
  displayName?: string,
): UserProfileSyncData {
  const name =
    displayName?.trim() || email.split("@")[0] || "ScoutIQ";
  return {
    id: userId,
    userId,
    displayName: name,
    email,
    language: "de",
    plan: "free",
    initials: initialsFromName(name),
    theme: "dark",
    updatedAt: nowIso(),
  };
}

export function defaultSettings(userId: string): AppSettingsSyncData {
  return {
    id: userId,
    userId,
    themePreference: "dark",
    language: "de",
    notificationsEnabled: true,
    updatedAt: nowIso(),
  };
}
