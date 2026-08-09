import type { ImportFile, ImportFormat } from "./types";
import { failImport } from "./normalizePlayer";
import type { ImportResult } from "./types";

function formatFromName(fileName: string): ImportFormat | null {
  const lower = fileName.toLowerCase();
  if (lower.endsWith(".csv")) return "csv";
  if (lower.endsWith(".json")) return "json";
  if (lower.endsWith(".fm") || lower.endsWith(".sav")) return "fm_save";
  return null;
}

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function pickWithTauri(): Promise<ImportFile | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const { readTextFile } = await import("@tauri-apps/plugin-fs");

  const selected = await open({
    multiple: false,
    directory: false,
    title: "Daten importieren",
    filters: [
      {
        name: "ScoutIQ Import",
        extensions: ["csv", "json"],
      },
    ],
  });

  if (!selected || Array.isArray(selected)) return null;

  const path = selected;
  const name = path.split(/[/\\]/).pop() ?? "import";
  const format = formatFromName(name);
  if (!format || (format !== "csv" && format !== "json")) {
    throw Object.assign(new Error("unsupported_format"), {
      code: "unsupported_format" as const,
    });
  }

  const content = await readTextFile(path);
  return { name, path, content, format };
}

function pickWithBrowser(): Promise<ImportFile | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".csv,.json,text/csv,application/json";
    input.style.display = "none";

    input.addEventListener("change", () => {
      const file = input.files?.[0];
      document.body.removeChild(input);
      if (!file) {
        resolve(null);
        return;
      }

      const format = formatFromName(file.name);
      if (!format || (format !== "csv" && format !== "json")) {
        reject(
          Object.assign(new Error("unsupported_format"), {
            code: "unsupported_format" as const,
          }),
        );
        return;
      }

      const reader = new FileReader();
      reader.onload = () => {
        resolve({
          name: file.name,
          content: String(reader.result ?? ""),
          format,
        });
      };
      reader.onerror = () => {
        reject(
          Object.assign(new Error("read_failed"), {
            code: "read_failed" as const,
          }),
        );
      };
      reader.readAsText(file);
    });

    document.body.appendChild(input);
    input.click();
  });
}

/** Native desktop picker in Tauri; browser file input as fallback. */
export async function pickImportFile(): Promise<
  | { ok: true; file: ImportFile }
  | { ok: false; result: ImportResult }
> {
  try {
    const file = isTauriRuntime()
      ? await pickWithTauri()
      : await pickWithBrowser();

    if (!file) {
      return { ok: false, result: failImport("cancelled") };
    }

    if (!file.content.trim()) {
      return { ok: false, result: failImport("empty_file") };
    }

    return { ok: true, file };
  } catch (error) {
    const code =
      error && typeof error === "object" && "code" in error
        ? (error as { code?: string }).code
        : undefined;

    if (code === "unsupported_format") {
      return { ok: false, result: failImport("unsupported_format") };
    }
    if (code === "read_failed") {
      return { ok: false, result: failImport("read_failed") };
    }

    return {
      ok: false,
      result: failImport(
        "read_failed",
        error instanceof Error ? error.message : undefined,
      ),
    };
  }
}
