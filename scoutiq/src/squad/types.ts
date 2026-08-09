export type SquadStatus =
  | "first_team"
  | "rotation"
  | "backup"
  | "youth"
  | "loan"
  | "listed"
  | "unknown";

export interface Player {
  id: string;
  name: string;
  position: string;
  age: number;
  nationality: string;
  marketValue: string;
  wage: string;
  contractExpiry: string;
  squadStatus: SquadStatus;
}

export interface Squad {
  id: string;
  clubName: string;
  season?: string;
  importedAt: string;
  sourceFormat: "csv" | "json" | "fm_save";
  sourceFileName: string;
  players: Player[];
}

export type PlayerSortKey =
  | "name"
  | "position"
  | "age"
  | "nationality"
  | "marketValue"
  | "wage"
  | "contractExpiry"
  | "squadStatus";

export type SortDirection = "asc" | "desc";
