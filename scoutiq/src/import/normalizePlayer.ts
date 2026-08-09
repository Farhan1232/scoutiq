import type { Player, SquadStatus } from "../squad/types";
import type { NormalizedPlayerInput } from "./types";

const FIELD_ALIASES: Record<keyof Omit<Player, "id">, string[]> = {
  name: ["name", "player", "playername", "spieler", "spielername", "name_"],
  position: ["position", "pos", "role", "rolle"],
  age: ["age", "alter"],
  nationality: [
    "nationality",
    "nation",
    "nat",
    "nationalitaet",
    "nationalität",
    "country",
    "land",
  ],
  marketValue: [
    "marketvalue",
    "market_value",
    "value",
    "marktwert",
    "transferwert",
  ],
  wage: ["wage", "salary", "gehalt", "wochengehalt", "wages"],
  contractExpiry: [
    "contractexpiry",
    "contract_expiry",
    "contract",
    "expires",
    "expiry",
    "vertrag",
    "vertragsende",
    "vertragbis",
    "vertrag_bis",
  ],
  squadStatus: [
    "squadstatus",
    "squad_status",
    "status",
    "kaderstatus",
    "playingtime",
    "playing_time",
  ],
};

const STATUS_MAP: Record<string, SquadStatus> = {
  first_team: "first_team",
  firstteam: "first_team",
  "first team": "first_team",
  stammkraft: "first_team",
  stamm: "first_team",
  important: "first_team",
  star: "first_team",
  rotation: "rotation",
  regular: "rotation",
  rotationer: "rotation",
  backup: "backup",
  reserve: "backup",
  ersatz: "backup",
  youth: "youth",
  u21: "youth",
  u19: "youth",
  jugend: "youth",
  loan: "loan",
  ausgeliehen: "loan",
  listed: "listed",
  transferlisted: "listed",
  verkaufsliste: "listed",
  unknown: "unknown",
  unbekannt: "unknown",
};

function normalizeKey(key: string): string {
  return key
    .trim()
    .toLowerCase()
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9]+/g, "");
}

function pickField(
  row: Record<string, unknown>,
  aliases: string[],
): unknown {
  const normalized = new Map(
    Object.entries(row).map(([key, value]) => [normalizeKey(key), value]),
  );

  for (const alias of aliases) {
    const value = normalized.get(normalizeKey(alias));
    if (value !== undefined && value !== null && String(value).trim() !== "") {
      return value;
    }
  }
  return undefined;
}

function asString(value: unknown, fallback = "—"): string {
  if (value === undefined || value === null) return fallback;
  const text = String(value).trim();
  return text || fallback;
}

function asAge(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return Math.round(value);
  }
  const text = String(value ?? "").trim().replace(",", ".");
  if (!text) return null;
  const parsed = Number.parseFloat(text);
  if (!Number.isFinite(parsed)) return null;
  return Math.round(parsed);
}

function asStatus(value: unknown): SquadStatus {
  const key = String(value ?? "")
    .trim()
    .toLowerCase()
    .replace(/[_-]+/g, " ");
  return STATUS_MAP[key] ?? (key ? "unknown" : "unknown");
}

export function normalizePlayerRow(
  input: NormalizedPlayerInput | Record<string, unknown>,
  index: number,
): Player | null {
  const name = asString(
    pickField(input, FIELD_ALIASES.name) ?? input.name,
    "",
  );
  const age = asAge(pickField(input, FIELD_ALIASES.age) ?? input.age);

  if (!name || age === null || age < 14 || age > 60) {
    return null;
  }

  return {
    id: `player-${index + 1}-${normalizeKey(name).slice(0, 24) || index}`,
    name,
    position: asString(
      pickField(input, FIELD_ALIASES.position) ?? input.position,
      "—",
    ),
    age,
    nationality: asString(
      pickField(input, FIELD_ALIASES.nationality) ?? input.nationality,
      "—",
    ),
    marketValue: asString(
      pickField(input, FIELD_ALIASES.marketValue) ?? input.marketValue,
      "—",
    ),
    wage: asString(pickField(input, FIELD_ALIASES.wage) ?? input.wage, "—"),
    contractExpiry: asString(
      pickField(input, FIELD_ALIASES.contractExpiry) ?? input.contractExpiry,
      "—",
    ),
    squadStatus: asStatus(
      pickField(input, FIELD_ALIASES.squadStatus) ?? input.squadStatus,
    ),
  };
}

export function failImport(
  code: import("./types").ImportErrorCode,
  detail?: string,
): import("./types").ImportResult {
  return {
    ok: false,
    error: {
      code,
      messageKey: `import.errors.${code}`,
      detail,
    },
  };
}
