/** Global current-save identity — one source of truth for club/league UI. */

export type SaveContextStatus =
  | "none"
  | "loading"
  | "partial"
  | "ready"
  | "error";

export type FieldConfidence =
  | "confirmed"
  | "high"
  | "medium"
  | "low"
  | "unknown";

export interface CurrentSaveEntity {
  id: string | null;
  name: string | null;
  shortName?: string | null;
}

/**
 * Where the competition's display name came from. The parser resolves the
 * competition **uid** and its **name** in two separate steps, so a name may be
 * missing while the uid is perfectly valid.
 */
export type CompetitionNameSource =
  | "saveShortName"
  | "saveFullName"
  | "standardFm26Catalogue"
  | "saveAbbreviation"
  | "unavailable";

export interface CurrentSaveConfidence {
  manager: FieldConfidence;
  club: FieldConfidence;
  competition: FieldConfidence;
  season: FieldConfidence;
  date: FieldConfidence;
  overall: FieldConfidence;
}

export interface CurrentSaveContext {
  filePath: string | null;
  fileName: string | null;
  fileSize: number | null;

  manager: CurrentSaveEntity;
  club: CurrentSaveEntity & { logoId?: string | null; externalId?: string | null };
  competition: CurrentSaveEntity;
  competitionNameSource: CompetitionNameSource;
  nation: CurrentSaveEntity;

  season: string | null;
  inGameDate: string | null;

  status: SaveContextStatus;
  confidence: CurrentSaveConfidence;

  parserEngineVersion: string | null;
  parserSchemaVersion: number | null;
  fingerprint: string | null;
  warnings: string[];
  relationshipSummary: string | null;
  evidence: Array<{ kind: string; detail: string }>;
  errorMessage: string | null;
}

export const EMPTY_CURRENT_SAVE: CurrentSaveContext = {
  filePath: null,
  fileName: null,
  fileSize: null,
  manager: { id: null, name: null },
  club: { id: null, name: null, shortName: null, logoId: null, externalId: null },
  competition: { id: null, name: null, shortName: null },
  competitionNameSource: "unavailable",
  nation: { id: null, name: null },
  season: null,
  inGameDate: null,
  status: "none",
  confidence: {
    manager: "unknown",
    club: "unknown",
    competition: "unknown",
    season: "unknown",
    date: "unknown",
    overall: "unknown",
  },
  parserEngineVersion: null,
  parserSchemaVersion: null,
  fingerprint: null,
  warnings: [],
  relationshipSummary: null,
  evidence: [],
  errorMessage: null,
};

/** Confirmed / High → show as fact. */
export function isUserFacingConfidence(c: FieldConfidence): boolean {
  return c === "confirmed" || c === "high";
}

/** Medium → show only with "possibly detected" label. */
export function isPossibleConfidence(c: FieldConfidence): boolean {
  return c === "medium";
}

export function displayClubName(ctx: CurrentSaveContext): string | null {
  if (ctx.status === "loading" || ctx.status === "none") return null;
  if (isUserFacingConfidence(ctx.confidence.club) || isPossibleConfidence(ctx.confidence.club)) {
    return ctx.club.name ?? ctx.club.shortName ?? null;
  }
  return null;
}

function competitionIsTrusted(ctx: CurrentSaveContext): boolean {
  if (ctx.status === "loading" || ctx.status === "none") return false;
  return (
    isUserFacingConfidence(ctx.confidence.competition) ||
    isPossibleConfidence(ctx.confidence.competition)
  );
}

export function displayCompetitionName(ctx: CurrentSaveContext): string | null {
  if (!competitionIsTrusted(ctx)) return null;
  return ctx.competition.shortName ?? ctx.competition.name ?? null;
}

/**
 * The competition's uid when it resolved but no display name is available —
 * FM stores a competition's name only when it differs from the installed
 * database's default, so a valid league can legitimately have no label in the
 * file. Callers show this as "Competition {id}" rather than "not identified",
 * because the league *was* identified; only its name is missing.
 */
export function competitionIdWithoutName(
  ctx: CurrentSaveContext,
): string | null {
  if (!competitionIsTrusted(ctx)) return null;
  if (displayCompetitionName(ctx)) return null;
  return ctx.competition.id;
}

export function displaySeason(ctx: CurrentSaveContext): string | null {
  if (
    isUserFacingConfidence(ctx.confidence.season) ||
    isPossibleConfidence(ctx.confidence.season)
  ) {
    return ctx.season;
  }
  return null;
}
