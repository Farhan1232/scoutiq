import type { UserProfileSyncData } from "../types";
import type { ProfileSyncService } from "./types";
import { getCloudProvider } from "../syncEngine";

/** Public service facade for user profile sync. */
export function getProfileSyncService(): ProfileSyncService {
  return getCloudProvider().profile;
}

export async function readLocalProfile(): Promise<UserProfileSyncData | null> {
  return getCloudProvider().profile.getLocal();
}
