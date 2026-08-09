import { detectImporter, isPhase1Format } from "./importers";
import { pickImportFile } from "./filePicker";
import { failImport } from "./normalizePlayer";
import type { ImportResult } from "./types";

/**
 * Orchestrates file picking + format detection + parsing.
 * UI components should only call this service.
 */
export async function importSquadFromFile(): Promise<ImportResult> {
  const picked = await pickImportFile();
  if (!picked.ok) return picked.result;

  const { file } = picked;
  if (!isPhase1Format(file.format)) {
    return failImport("unsupported_format");
  }

  const importer = detectImporter(file.name);
  if (!importer || !isPhase1Format(importer.id)) {
    return failImport("unsupported_format");
  }

  try {
    return await importer.parse(file);
  } catch (error) {
    return failImport(
      "unknown",
      error instanceof Error ? error.message : undefined,
    );
  }
}
