import type { FmSaveRef, SaveError, ScoutIqProject } from "./types";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function fileNameFromPath(path: string): string {
  return path.split(/[/\\]/).pop() ?? path;
}

export function createEmptyProject(
  name: string,
  fmSave: FmSaveRef | null = null,
): ScoutIqProject {
  const now = new Date().toISOString();
  return {
    version: 1,
    name,
    createdAt: now,
    updatedAt: now,
    fmSave,
    analyses: [],
    importedMetadata: {},
    notes: [],
    shortlists: [],
    settings: {},
    squadSnapshot: null,
  };
}

export type ProjectWriteResult =
  | { ok: true; path: string; project: ScoutIqProject }
  | { ok: false; error: SaveError };

export async function saveScoutIqProject(
  project: ScoutIqProject,
  existingPath: string | null,
): Promise<ProjectWriteResult> {
  if (!isTauriRuntime()) {
    return {
      ok: false,
      error: { code: "unknown", messageKey: "save.errors.desktopOnly" },
    };
  }

  try {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const { writeTextFile } = await import("@tauri-apps/plugin-fs");

    let path = existingPath;
    if (!path) {
      const picked = await save({
        title: "ScoutIQ-Projekt speichern unter",
        defaultPath: `${project.name || "ScoutIQ"}.scoutiq.json`,
        filters: [
          {
            name: "ScoutIQ Projekt",
            extensions: ["scoutiq.json", "json"],
          },
        ],
      });
      if (!picked) {
        return {
          ok: false,
          error: { code: "cancelled", messageKey: "save.errors.cancelled" },
        };
      }
      path = picked.endsWith(".json") ? picked : `${picked}.scoutiq.json`;
    }

    const next: ScoutIqProject = {
      ...project,
      updatedAt: new Date().toISOString(),
      name: project.name || fileNameFromPath(path).replace(/\.scoutiq\.json$/i, ""),
    };

    // Never writes to the original FM save path — only ScoutIQ project files.
    await writeTextFile(path, JSON.stringify(next, null, 2));
    return { ok: true, path, project: next };
  } catch (error) {
    return {
      ok: false,
      error: {
        code: "write_failed",
        messageKey: "save.errors.writeFailed",
        detail: error instanceof Error ? error.message : undefined,
      },
    };
  }
}

export async function saveScoutIqProjectAs(
  project: ScoutIqProject,
): Promise<ProjectWriteResult> {
  return saveScoutIqProject(project, null);
}
