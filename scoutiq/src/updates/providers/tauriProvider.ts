import { areUpdatesEnabled } from "../config";
import { readAppVersion } from "../version";
import type { UpdateCheckResult, UpdateProvider } from "../types";

/**
 * Real Tauri updater provider (prepared, not active).
 *
 * To enable later:
 * 1. Set UPDATES_ENABLED = true in ../config.ts
 * 2. npm i @tauri-apps/plugin-updater @tauri-apps/plugin-process
 * 3. Apply src-tauri/updater.example.json + capability permissions
 * 4. Register tauri_plugin_updater in src-tauri/src/lib.rs
 * 5. Replace checkForUpdates() body with the plugin call below
 *
 * Intended implementation:
 *   import { check } from "@tauri-apps/plugin-updater";
 *   const update = await check();
 *   if (!update) return upToDate;
 *   return available(update.version, update.body, update.date);
 */
export const tauriUpdateProvider: UpdateProvider = {
  id: "tauri",

  async getCurrentVersion() {
    return readAppVersion();
  },

  async checkForUpdates(): Promise<UpdateCheckResult> {
    const currentVersion = await readAppVersion();

    if (!areUpdatesEnabled()) {
      return {
        status: "disabled",
        currentVersion,
        messageKey: "settings.updates.status.disabled",
      };
    }

    return {
      status: "error",
      currentVersion,
      messageKey: "settings.updates.status.notConfigured",
    };
  },
};
