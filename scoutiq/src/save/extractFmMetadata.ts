import { invoke } from "@tauri-apps/api/core";
import type { FmSaveMetadata } from "./fmMetadata";
import type { SaveError } from "./types";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export type ExtractFmMetadataResult =
  | { ok: true; metadata: FmSaveMetadata }
  | { ok: false; error: SaveError };

/** @deprecated Prefer analyzeFmSave — kept for direct debugging. */
export async function extractFmMetadata(
  path: string,
): Promise<ExtractFmMetadataResult> {
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
    const metadata = await invoke<FmSaveMetadata>("extract_fm_metadata", { path });
    return { ok: true, metadata };
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
