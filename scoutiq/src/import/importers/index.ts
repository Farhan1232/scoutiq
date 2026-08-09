import type { DataImporter, ImportFormat } from "../types";
import { csvImporter } from "./csvImporter";
import { fmSaveImporter } from "./fmSaveImporter";
import { jsonImporter } from "./jsonImporter";

/** Phase-1 importers. FM save is registered but returns not_implemented. */
export const PHASE1_IMPORTERS: DataImporter[] = [csvImporter, jsonImporter];

export const ALL_IMPORTERS: DataImporter[] = [
  ...PHASE1_IMPORTERS,
  fmSaveImporter,
];

export function detectImporter(fileName: string): DataImporter | null {
  return ALL_IMPORTERS.find((importer) => importer.canHandle(fileName)) ?? null;
}

export function isPhase1Format(format: ImportFormat): boolean {
  return format === "csv" || format === "json";
}
