/** Read-only reference to an original Football Manager save. Never mutate the FM file. */
export interface FmSaveRef {
  path: string;
  fileName: string;
  fileSizeBytes: number;
  lastModifiedAt: string;
  loadedAt: string;
}

export type SaveLoadState =
  | "none"
  | "loading"
  | "loaded"
  | "error";

export interface RecentFmSave {
  path: string;
  fileName: string;
  openedAt: string;
}

/** Separate ScoutIQ project — never overwrites the FM save. */
export interface ScoutIqProject {
  version: 1;
  name: string;
  createdAt: string;
  updatedAt: string;
  /** Reference only — original FM save remains untouched. */
  fmSave: FmSaveRef | null;
  analyses: unknown[];
  importedMetadata: Record<string, unknown>;
  notes: unknown[];
  shortlists: unknown[];
  settings: Record<string, unknown>;
  /** Optional snapshot of imported squad metadata (not FM blobs). */
  squadSnapshot: unknown | null;
}

export interface SaveError {
  code: "cancelled" | "unsupported" | "read_failed" | "write_failed" | "unknown";
  messageKey: string;
  detail?: string;
}
