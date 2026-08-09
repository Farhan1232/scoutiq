/**
 * Cloud sync configuration for ScoutIQ.
 * Supabase is the primary cloud source for signed-in users.
 * localStorage is used only as an offline cache.
 */

export type CloudBackend = "local" | "supabase";

export const CLOUD_BACKEND: CloudBackend = "supabase";

/** Master switch — real Supabase sync is enabled. */
export const CLOUD_SYNC_ENABLED = true;

export const CLOUD_STORAGE_PREFIX = "scoutiq.cloud.";

export const SYNC_DOMAINS = [
  "profile",
  "settings",
  "saveMetadata",
  "analysisHistory",
  "transferShortlist",
] as const;

export type SyncDomain = (typeof SYNC_DOMAINS)[number];

export function isCloudSyncEnabled(): boolean {
  return CLOUD_SYNC_ENABLED && CLOUD_BACKEND === "supabase";
}
