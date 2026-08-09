import { useTranslation } from "react-i18next";
import type { Player, PlayerSortKey, SortDirection } from "../../squad/types";
import { Badge } from "../ui/Badge";
import "./SquadTable.css";

interface SquadTableProps {
  players: Player[];
  sortKey: PlayerSortKey;
  sortDirection: SortDirection;
  onSort: (key: PlayerSortKey) => void;
}

const COLUMNS: { key: PlayerSortKey; labelKey: string }[] = [
  { key: "name", labelKey: "squad.table.name" },
  { key: "position", labelKey: "squad.table.position" },
  { key: "age", labelKey: "squad.table.age" },
  { key: "nationality", labelKey: "squad.table.nationality" },
  { key: "marketValue", labelKey: "squad.table.marketValue" },
  { key: "wage", labelKey: "squad.table.wage" },
  { key: "contractExpiry", labelKey: "squad.table.contractExpiry" },
  { key: "squadStatus", labelKey: "squad.table.squadStatus" },
];

function statusTone(status: Player["squadStatus"]) {
  if (status === "first_team") return "accent" as const;
  if (status === "listed") return "warning" as const;
  if (status === "loan") return "info" as const;
  return "neutral" as const;
}

export function SquadTable({
  players,
  sortKey,
  sortDirection,
  onSort,
}: SquadTableProps) {
  const { t } = useTranslation();

  return (
    <div className="squad-table-wrap">
      <table className="squad-table">
        <thead>
          <tr>
            {COLUMNS.map((column) => {
              const active = sortKey === column.key;
              return (
                <th key={column.key}>
                  <button
                    type="button"
                    className={`squad-table__sort${active ? " is-active" : ""}`}
                    onClick={() => onSort(column.key)}
                  >
                    {t(column.labelKey)}
                    <span aria-hidden="true">
                      {active ? (sortDirection === "asc" ? " ↑" : " ↓") : ""}
                    </span>
                  </button>
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {players.map((player) => (
            <tr key={player.id}>
              <td className="squad-table__name">{player.name}</td>
              <td>{player.position}</td>
              <td>{player.age}</td>
              <td>{player.nationality}</td>
              <td>{player.marketValue}</td>
              <td>{player.wage}</td>
              <td>{player.contractExpiry}</td>
              <td>
                <Badge tone={statusTone(player.squadStatus)}>
                  {t(`squad.status.${player.squadStatus}`)}
                </Badge>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
