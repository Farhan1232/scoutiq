import type { DataImporter, ImportFile, ImportResult } from "../types";
import { failImport, normalizePlayerRow } from "../normalizePlayer";

function parseCsv(content: string): string[][] {
  const rows: string[][] = [];
  let current = "";
  let row: string[] = [];
  let inQuotes = false;

  for (let i = 0; i < content.length; i += 1) {
    const char = content[i];
    const next = content[i + 1];

    if (char === '"' && inQuotes && next === '"') {
      current += '"';
      i += 1;
      continue;
    }

    if (char === '"') {
      inQuotes = !inQuotes;
      continue;
    }

    if (char === "," && !inQuotes) {
      row.push(current.trim());
      current = "";
      continue;
    }

    if ((char === "\n" || char === "\r") && !inQuotes) {
      if (char === "\r" && next === "\n") i += 1;
      row.push(current.trim());
      if (row.some((cell) => cell !== "")) rows.push(row);
      row = [];
      current = "";
      continue;
    }

    current += char;
  }

  row.push(current.trim());
  if (row.some((cell) => cell !== "")) rows.push(row);
  return rows;
}

export const csvImporter: DataImporter = {
  id: "csv",
  labelKey: "import.formats.csv",
  canHandle(fileName) {
    return fileName.toLowerCase().endsWith(".csv");
  },
  async parse(file: ImportFile): Promise<ImportResult> {
    const rows = parseCsv(file.content);
    if (rows.length < 2) {
      return failImport("invalid_csv");
    }

    const headers = rows[0];
    if (headers.length < 2) {
      return failImport("invalid_csv");
    }

    const players = [];
    let invalidRows = 0;

    for (let i = 1; i < rows.length; i += 1) {
      const values = rows[i];
      const record: Record<string, unknown> = {};
      headers.forEach((header, index) => {
        record[header] = values[index] ?? "";
      });

      const player = normalizePlayerRow(record, players.length);
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
        clubName: "Importierter Kader",
        importedAt: new Date().toISOString(),
        sourceFormat: "csv",
        sourceFileName: file.name,
        players,
      },
    };
  },
};
