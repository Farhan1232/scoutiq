import type { FmSaveRef, SaveError } from "./types";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function fileNameFromPath(path: string): string {
  return path.split(/[/\\]/).pop() ?? path;
}

export type OpenFmSaveResult =
  | { ok: true; save: FmSaveRef }
  | { ok: false; error: SaveError };

/**
 * Opens a native file picker and returns the path immediately.
 * Does NOT call fs.stat — that can hang forever on iCloud placeholders.
 * Size/mtime come from the Rust analyzer instead.
 */
export async function openFmSaveWithPicker(): Promise<OpenFmSaveResult> {
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
    const { open } = await import("@tauri-apps/plugin-dialog");

    const selected = await open({
      multiple: false,
      directory: false,
      title: "FM-Spielstand öffnen",
      filters: [
        {
          name: "Football Manager Spielstand",
          extensions: ["fm", "sav"],
        },
      ],
    });

    if (!selected || Array.isArray(selected)) {
      return {
        ok: false,
        error: { code: "cancelled", messageKey: "save.errors.cancelled" },
      };
    }

    return pathToSaveRef(selected);
  } catch (error) {
    return {
      ok: false,
      error: {
        code: "read_failed",
        messageKey: "save.errors.readFailed",
        detail: error instanceof Error ? error.message : undefined,
      },
    };
  }
}

/** Build a save ref from a path without touching the filesystem from JS. */
export function pathToSaveRef(path: string): OpenFmSaveResult {
  const fileName = fileNameFromPath(path);
  return {
    ok: true,
    save: {
      path,
      fileName,
      fileSizeBytes: 0,
      lastModifiedAt: new Date().toISOString(),
      loadedAt: new Date().toISOString(),
    },
  };
}

/** Recent-files reopen — also avoids fs.stat to prevent iCloud hangs. */
export async function loadFmSaveMetadata(
  path: string,
): Promise<OpenFmSaveResult> {
  if (!path.trim()) {
    return {
      ok: false,
      error: {
        code: "read_failed",
        messageKey: "save.errors.readFailed",
      },
    };
  }
  return pathToSaveRef(path);
}
