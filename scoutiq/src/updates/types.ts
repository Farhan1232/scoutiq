export type UpdateStatusKind =
  | "idle"
  | "checking"
  | "upToDate"
  | "available"
  | "disabled"
  | "error";

export interface UpdateAvailability {
  version: string;
  notes?: string;
  date?: string;
}

export interface UpdateCheckResult {
  status: Exclude<UpdateStatusKind, "idle" | "checking">;
  currentVersion: string;
  available?: UpdateAvailability;
  messageKey: string;
  messageParams?: Record<string, string>;
}

export interface UpdateProvider {
  readonly id: "disabled" | "tauri";
  getCurrentVersion(): Promise<string>;
  checkForUpdates(): Promise<UpdateCheckResult>;
}
