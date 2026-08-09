import { areUpdatesEnabled } from "./config";
import { disabledUpdateProvider } from "./providers/disabledProvider";
import { tauriUpdateProvider } from "./providers/tauriProvider";
import type { UpdateCheckResult, UpdateProvider } from "./types";

export function getUpdateProvider(): UpdateProvider {
  return areUpdatesEnabled() ? tauriUpdateProvider : disabledUpdateProvider;
}

export async function getCurrentAppVersion(): Promise<string> {
  return getUpdateProvider().getCurrentVersion();
}

export async function checkForAppUpdates(): Promise<UpdateCheckResult> {
  return getUpdateProvider().checkForUpdates();
}
