import type { DataImporter, ImportFile, ImportResult } from "../types";
import { failImport, normalizePlayerRow } from "../normalizePlayer";

function extractPlayers(payload: unknown): {
  players: Record<string, unknown>[];
  clubName?: string;
  season?: string;
} | null {
  if (Array.isArray(payload)) {
    return { players: payload as Record<string, unknown>[] };
  }

  if (!payload || typeof payload !== "object") return null;

  const object = payload as Record<string, unknown>;
  const candidates = [object.players, object.squad, object.data, object.kader];
  for (const candidate of candidates) {
    if (Array.isArray(candidate)) {
      return {
        players: candidate as Record<string, unknown>[],
        clubName:
          typeof object.clubName === "string"
            ? object.clubName
            : typeof object.club === "string"
              ? object.club
              : undefined,
        season: typeof object.season === "string" ? object.season : undefined,
      };
    }
  }

  return null;
}

export const jsonImporter: DataImporter = {
  id: "json",
  labelKey: "import.formats.json",
  canHandle(fileName) {
    return fileName.toLowerCase().endsWith(".json");
  },
  async parse(file: ImportFile): Promise<ImportResult> {
    let payload: unknown;
    try {
      payload = JSON.parse(file.content);
    } catch {
      return failImport("invalid_json");
    }

    const extracted = extractPlayers(payload);
    if (!extracted) {
      return failImport("missing_players");
    }

    const players = [];
    let invalidRows = 0;

    for (const row of extracted.players) {
      const player = normalizePlayerRow(row, players.length);
      if (player) players.push(player);
      else invalidRows += 1;
    }

    if (players.length === 0) {
      return failImport(
        invalidRows > 0 ? "invalid_player_row" : "missing_players",
      );
    }

    return {
      ok: true,
      squad: {
        id: `squad-${Date.now()}`,
        clubName: extracted.clubName?.trim() || "Importierter Kader",
        season: extracted.season,
        importedAt: new Date().toISOString(),
        sourceFormat: "json",
        sourceFileName: file.name,
        players,
      },
    };
  },
};
