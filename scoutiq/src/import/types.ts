import type { Player, Squad } from "../squad/types";

export type ImportFormat = "csv" | "json" | "fm_save";

export type ImportErrorCode =
  | "cancelled"
  | "unsupported_format"
  | "empty_file"
  | "invalid_json"
  | "invalid_csv"
  | "missing_players"
  | "invalid_player_row"
  | "read_failed"
  | "not_implemented"
  | "unknown";

export interface ImportError {
  code: ImportErrorCode;
  messageKey: string;
  detail?: string;
}

export type ImportResult =
  | { ok: true; squad: Squad }
  | { ok: false; error: ImportError };

export interface ImportFile {
  name: string;
  path?: string;
  content: string;
  format: ImportFormat;
}

/** Future-proof importer contract — FM save importers can plug in later. */
export interface DataImporter {
  readonly id: ImportFormat;
  readonly labelKey: string;
  canHandle(fileName: string): boolean;
  parse(file: ImportFile): Promise<ImportResult>;
}

export interface NormalizedPlayerInput {
  name?: unknown;
  position?: unknown;
  age?: unknown;
  nationality?: unknown;
  marketValue?: unknown;
  wage?: unknown;
  contractExpiry?: unknown;
  squadStatus?: unknown;
  [key: string]: unknown;
}

export type { Player, Squad };
