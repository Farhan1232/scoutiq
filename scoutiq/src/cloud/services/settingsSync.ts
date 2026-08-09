import type { AppSettingsSyncData } from "../types";
import type { SettingsSyncService } from "./types";
import { getCloudProvider } from "../syncEngine";

export function getSettingsSyncService(): SettingsSyncService {
  return getCloudProvider().settings;
}

export async function readLocalSettings(): Promise<AppSettingsSyncData | null> {
  return getCloudProvider().settings.getLocal();
}
