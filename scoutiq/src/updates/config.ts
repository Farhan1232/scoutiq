/**
 * Update architecture for ScoutIQ (Tauri updater-ready).
 *
 * Current state: updates are intentionally DISABLED.
 * Enabling later should require only:
 * 1. Set UPDATES_ENABLED = true in config.ts
 * 2. Install @tauri-apps/plugin-updater (+ process) and tauri-plugin-updater
 * 3. Apply src-tauri/updater.example.json into tauri.conf.json
 * 4. Grant updater permissions in capabilities
 * 5. Register the Rust plugin in lib.rs
 */

export const UPDATES_ENABLED = false;

/** Fallback when Tauri getVersion() is unavailable (e.g. browser preview). */
export const APP_VERSION_FALLBACK = "0.1.0";

/**
 * Feature gate used by the service layer.
 * Keep this as the single switch for enabling real update checks.
 */
export function areUpdatesEnabled(): boolean {
  return UPDATES_ENABLED;
}
