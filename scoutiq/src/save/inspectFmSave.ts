import { invoke } from "@tauri-apps/api/core";
import type { FmSaveInspection } from "./fmInspection";
import type { SaveError } from "./types";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export type InspectFmSaveResult =
  | { ok: true; inspection: FmSaveInspection }
  | { ok: false; error: SaveError };

/** @deprecated Prefer analyzeFmSave — kept for direct debugging. */
export async function inspectFmSave(path: string): Promise<InspectFmSaveResult> {
  if (!isTauriRuntime()) {
    return {
      ok: false,
      error: {
        code: "unknown",
        messageKey: "save.errors.desktopOnly",
      },
    };
  }

  try {
    const inspection = await invoke<FmSaveInspection>("inspect_fm_save", { path });
    return { ok: true, inspection };
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    return {
      ok: false,
      error: {
        code: "read_failed",
        messageKey: "save.errors.inspectFailed",
        detail,
      },
    };
  }
}
