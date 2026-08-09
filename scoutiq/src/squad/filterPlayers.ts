import type { Player, PlayerSortKey, SortDirection } from "./types";

function compareText(a: string, b: string): number {
  return a.localeCompare(b, "de", { sensitivity: "base" });
}

function parseLooseNumber(value: string): number | null {
  const cleaned = value
    .replace(/\s/g, "")
    .replace(/€|eur|£|\$/gi, "")
    .replace(/\./g, "")
    .replace(",", ".");
  const match = cleaned.match(/-?\d+(\.\d+)?/);
  if (!match) return null;
  const num = Number.parseFloat(match[0]);
  return Number.isFinite(num) ? num : null;
}

export function filterAndSortPlayers(
  players: Player[],
  options: {
    query: string;
    position: string;
    sortKey: PlayerSortKey;
    sortDirection: SortDirection;
  },
): Player[] {
  const query = options.query.trim().toLowerCase();
  const position = options.position.trim().toLowerCase();

  let result = players.filter((player) => {
    const matchesQuery =
      !query ||
      player.name.toLowerCase().includes(query) ||
      player.nationality.toLowerCase().includes(query) ||
      player.position.toLowerCase().includes(query);

    const matchesPosition =
      !position ||
      position === "all" ||
      player.position.toLowerCase() === position;

    return matchesQuery && matchesPosition;
  });

  result = [...result].sort((a, b) => {
    let cmp = 0;
    switch (options.sortKey) {
      case "age":
        cmp = a.age - b.age;
        break;
      case "marketValue": {
        const av = parseLooseNumber(a.marketValue);
        const bv = parseLooseNumber(b.marketValue);
        if (av !== null && bv !== null) cmp = av - bv;
        else cmp = compareText(a.marketValue, b.marketValue);
        break;
      }
      case "wage": {
        const av = parseLooseNumber(a.wage);
        const bv = parseLooseNumber(b.wage);
        if (av !== null && bv !== null) cmp = av - bv;
        else cmp = compareText(a.wage, b.wage);
        break;
      }
      default:
        cmp = compareText(String(a[options.sortKey]), String(b[options.sortKey]));
    }
    return options.sortDirection === "asc" ? cmp : -cmp;
  });

  return result;
}

export function uniquePositions(players: Player[]): string[] {
  return [...new Set(players.map((player) => player.position).filter(Boolean))]
    .sort((a, b) => compareText(a, b));
}
