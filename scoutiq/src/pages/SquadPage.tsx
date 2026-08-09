import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { SquadTable } from "../components/squad/SquadTable";
import { Icon } from "../components/icons/Icon";
import { Button } from "../components/ui/Button";
import { filterAndSortPlayers, uniquePositions } from "../squad/filterPlayers";
import { useSquad } from "../squad/SquadContext";
import type { PlayerSortKey, SortDirection } from "../squad/types";
import "./SquadPage.css";

export function SquadPage() {
  const { t } = useTranslation();
  const { squad, isImporting, lastError, importSquad, clearImportedSquad, clearError } =
    useSquad();

  const [query, setQuery] = useState("");
  const [position, setPosition] = useState("all");
  const [sortKey, setSortKey] = useState<PlayerSortKey>("name");
  const [sortDirection, setSortDirection] = useState<SortDirection>("asc");

  const positions = useMemo(
    () => (squad ? uniquePositions(squad.players) : []),
    [squad],
  );

  const players = useMemo(() => {
    if (!squad) return [];
    return filterAndSortPlayers(squad.players, {
      query,
      position,
      sortKey,
      sortDirection,
    });
  }, [squad, query, position, sortKey, sortDirection]);

  const onSort = (key: PlayerSortKey) => {
    if (sortKey === key) {
      setSortDirection((value) => (value === "asc" ? "desc" : "asc"));
      return;
    }
    setSortKey(key);
    setSortDirection("asc");
  };

  if (!squad) {
    return (
      <div className="squad-page">
        <header className="squad-page__header">
          <div>
            <h2>{t("squad.title")}</h2>
            <p>{t("squad.subtitle")}</p>
          </div>
          <Button
            variant="primary"
            disabled={isImporting}
            onClick={() => {
              void importSquad();
            }}
          >
            <Icon name="upload" size={15} />
            {isImporting ? t("squad.importing") : t("squad.import")}
          </Button>
        </header>

        {lastError ? (
          <p className="squad-page__error">
            {t(lastError.messageKey)}
            {lastError.detail ? ` (${lastError.detail})` : ""}
          </p>
        ) : null}

        <div className="squad-page__empty">
          <div className="squad-page__empty-icon">
            <Icon name="squad" size={22} />
          </div>
          <h3>{t("squad.empty.title")}</h3>
          <p>{t("squad.empty.description")}</p>
          <Button
            variant="secondary"
            disabled={isImporting}
            onClick={() => {
              void importSquad();
            }}
          >
            <Icon name="upload" size={15} />
            {t("squad.import")}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="squad-page">
      <header className="squad-page__header">
        <div>
          <h2>{t("squad.title")}</h2>
          <p>
            {t("squad.loaded", {
              club: squad.clubName,
              count: squad.players.length,
              file: squad.sourceFileName,
            })}
          </p>
        </div>
        <div className="squad-page__actions">
          <Button
            variant="secondary"
            disabled={isImporting}
            onClick={() => {
              void importSquad();
            }}
          >
            <Icon name="upload" size={15} />
            {isImporting ? t("squad.importing") : t("squad.import")}
          </Button>
          <Button variant="ghost" onClick={clearImportedSquad}>
            {t("squad.clear")}
          </Button>
        </div>
      </header>

      {lastError ? (
        <p className="squad-page__error">
          {t(lastError.messageKey)}
          <button type="button" className="squad-page__error-dismiss" onClick={clearError}>
            {t("squad.dismissError")}
          </button>
        </p>
      ) : null}

      <div className="squad-page__toolbar">
        <label className="squad-page__search">
          <Icon name="search" size={15} />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t("squad.searchPlaceholder")}
            aria-label={t("squad.searchPlaceholder")}
          />
        </label>

        <label className="squad-page__filter">
          <span>{t("squad.filterPosition")}</span>
          <select
            value={position}
            onChange={(event) => setPosition(event.target.value)}
          >
            <option value="all">{t("squad.allPositions")}</option>
            {positions.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>
        </label>

        <span className="squad-page__count">
          {t("squad.resultCount", { count: players.length })}
        </span>
      </div>

      {players.length === 0 ? (
        <div className="squad-page__empty squad-page__empty--compact">
          <h3>{t("squad.noResults.title")}</h3>
          <p>{t("squad.noResults.description")}</p>
        </div>
      ) : (
        <SquadTable
          players={players}
          sortKey={sortKey}
          sortDirection={sortDirection}
          onSort={onSort}
        />
      )}
    </div>
  );
}
