export { UPDATES_ENABLED, areUpdatesEnabled, APP_VERSION_FALLBACK } from "./config";
export { getCurrentAppVersion, checkForAppUpdates, getUpdateProvider } from "./updateService";
export { useUpdates } from "./useUpdates";
export type {
  UpdateAvailability,
  UpdateCheckResult,
  UpdateProvider,
  UpdateStatusKind,
} from "./types";
