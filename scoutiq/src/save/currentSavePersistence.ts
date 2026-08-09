import type {
  CompetitionNameSource,
  CurrentSaveContext,
  FieldConfidence,
} from "./currentSaveTypes";
import { EMPTY_CURRENT_SAVE } from "./currentSaveTypes";

const STORAGE_KEY = "scoutiq.currentSave.v1";
/** Must match Rust PARSER_SCHEMA_VERSION for cache acceptance. */
export const EXPECTED_PARSER_SCHEMA_VERSION = 4;
/**
 * Must track Rust `PARSER_ENGINE_VERSION` (`entities/types.rs`). It read "0.3."
 * while the parser reported "0.4.0", so every cached identity was rejected and
 * each app start re-scanned the save from scratch.
 */
export const EXPECTED_PARSER_ENGINE_PREFIX = "0.4.";

export interface PersistedSaveIdentity {
  path: string;
  fileName: string;
  fileSize: number;
  fingerprint: string;
  modifiedUnix: number | null;
  context: CurrentSaveContext;
  savedAt: string;
}

export function loadPersistedIdentity(): PersistedSaveIdentity | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as PersistedSaveIdentity;
    if (
      !parsed?.path ||
      !parsed.context ||
      parsed.context.parserSchemaVersion !== EXPECTED_PARSER_SCHEMA_VERSION
    ) {
      return null;
    }
    const engine = parsed.context.parserEngineVersion ?? "";
    if (!engine.startsWith(EXPECTED_PARSER_ENGINE_PREFIX)) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export function persistIdentity(entry: PersistedSaveIdentity): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(entry));
  } catch {
    // ignore quota
  }
}

export function clearPersistedIdentity(): void {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // ignore
  }
}

export interface ManagedClubDto {
  managerId?: number | null;
  managerName?: string | null;
  clubId?: number | null;
  clubName?: string | null;
  clubShortName?: string | null;
  clubLogoId?: number | null;
  clubExternalId?: string | null;
  competitionId?: number | null;
  competitionName?: string | null;
  competitionShortName?: string | null;
  /** Rust `CompetitionNameSource`; independent of `competitionId`. */
  competitionNameSource?: string | null;
  nationId?: number | null;
  nationName?: string | null;
  season?: string | null;
  inGameDate?: string | null;
  confidence?: string;
  managerConfidence?: string;
  clubConfidence?: string;
  competitionConfidence?: string;
  seasonConfidence?: string;
  dateConfidence?: string;
  evidence?: Array<{ kind: string; detail: string }>;
  relationshipSummary?: string;
}

export interface FmSaveAnalysisResultDto {
  fileInfo: {
    fileName: string;
    filePath: string;
    fileSize: number;
    headerHex?: string;
    modifiedUnix?: number | null;
  };
  parserInfo: {
    engineVersion: string;
    gameVersion?: string | null;
    schemaVersion: number;
  };
  gameVersion?: {
    game?: string | null;
    majorVersion?: number | null;
    saveFormatVersion?: number | null;
  };
  managed: ManagedClubDto;
  warnings?: string[];
  confidence?: {
    manager: string;
    club: string;
    competition: string;
    season: string;
    date: string;
    overall: string;
  };
}

function conf(v: string | undefined): FieldConfidence {
  const x = (v ?? "unknown").toLowerCase();
  if (
    x === "confirmed" ||
    x === "high" ||
    x === "medium" ||
    x === "low" ||
    x === "unknown"
  ) {
    return x;
  }
  return "unknown";
}

function idStr(v: number | null | undefined): string | null {
  if (v === null || v === undefined) return null;
  return String(v);
}

const NAME_SOURCES: readonly CompetitionNameSource[] = [
  "saveShortName",
  "saveFullName",
  "standardFm26Catalogue",
  "saveAbbreviation",
  "unavailable",
];

function nameSource(v: string | null | undefined): CompetitionNameSource {
  const found = NAME_SOURCES.find((s) => s === v);
  return found ?? "unavailable";
}

export function identityToContext(
  identity: FmSaveAnalysisResultDto,
  status: CurrentSaveContext["status"] = "ready",
): CurrentSaveContext {
  const m = identity.managed ?? {};
  const c = identity.confidence;
  return {
    ...EMPTY_CURRENT_SAVE,
    filePath: identity.fileInfo.filePath,
    fileName: identity.fileInfo.fileName,
    fileSize: identity.fileInfo.fileSize,
    manager: {
      id: idStr(m.managerId),
      name: m.managerName ?? null,
    },
    club: {
      id: idStr(m.clubId),
      name: m.clubName ?? null,
      shortName: m.clubShortName ?? null,
      logoId: idStr(m.clubLogoId ?? null),
      externalId: m.clubExternalId ?? null,
    },
    competition: {
      id: idStr(m.competitionId),
      name: m.competitionName ?? null,
      shortName: m.competitionShortName ?? null,
    },
    competitionNameSource: nameSource(m.competitionNameSource),
    nation: {
      id: idStr(m.nationId),
      name: m.nationName ?? null,
    },
    season: m.season ?? null,
    inGameDate: m.inGameDate ?? null,
    status,
    confidence: {
      manager: conf(c?.manager ?? m.managerConfidence),
      club: conf(c?.club ?? m.clubConfidence),
      competition: conf(c?.competition ?? m.competitionConfidence),
      season: conf(c?.season ?? m.seasonConfidence),
      date: conf(c?.date ?? m.dateConfidence),
      overall: conf(c?.overall ?? m.confidence),
    },
    parserEngineVersion: identity.parserInfo.engineVersion,
    parserSchemaVersion: identity.parserInfo.schemaVersion,
    fingerprint: identity.fileInfo.headerHex
      ? `${identity.fileInfo.fileSize}:${identity.fileInfo.headerHex.slice(0, 64)}`
      : null,
    warnings: identity.warnings ?? [],
    relationshipSummary: m.relationshipSummary ?? null,
    evidence: (m.evidence ?? []).map((e) => ({
      kind: e.kind,
      detail: e.detail,
    })),
    errorMessage: null,
  };
}

export function loadingContext(file: {
  path: string;
  fileName: string;
  fileSizeBytes: number;
}): CurrentSaveContext {
  return {
    ...EMPTY_CURRENT_SAVE,
    filePath: file.path,
    fileName: file.fileName,
    fileSize: file.fileSizeBytes,
    status: "loading",
  };
}
