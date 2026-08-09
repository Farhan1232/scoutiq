import type { DataImporter, ImportResult } from "../types";
import { failImport } from "../normalizePlayer";

/**
 * Placeholder for future Football Manager save-file import (.fm / .sav).
 * Not enabled in phase 1.
 */
export const fmSaveImporter: DataImporter = {
  id: "fm_save",
  labelKey: "import.formats.fmSave",
  canHandle(fileName) {
    const lower = fileName.toLowerCase();
    return lower.endsWith(".fm") || lower.endsWith(".sav");
  },
  async parse(): Promise<ImportResult> {
    return failImport("not_implemented");
  },
};
