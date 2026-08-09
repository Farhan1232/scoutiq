import { useTranslation } from "react-i18next";
import { useAuth } from "../../auth";
import { useCurrentSave } from "../../save/SaveContext";
import {
  displayClubName,
  displayCompetitionName,
  displaySeason,
} from "../../save/currentSaveTypes";
import type { NavId } from "../../types";
import { CloudSyncIndicator } from "../cloud/CloudSyncIndicator";
import { SaveStatusBadge } from "../save/SaveStatusBadge";
import { Icon } from "../icons/Icon";
import { Button } from "../ui/Button";
import { UserMenu } from "./UserMenu";
import "./TopNav.css";

interface TopNavProps {
  active: NavId;
  title: string;
  onOpenProfile: () => void;
}

export function TopNav({ active, title, onOpenProfile }: TopNavProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const currentSave = useCurrentSave();
  const club = displayClubName(currentSave);
  const competition = displayCompetitionName(currentSave);
  const season = displaySeason(currentSave);

  const subtitle =
    active === "dashboard"
      ? club
        ? [
            competition ?? t("currentSave.competitionUnknown"),
            season,
            user?.displayName,
          ]
            .filter(Boolean)
            .join(" · ")
        : currentSave.status === "loading"
          ? t("currentSave.loading")
          : currentSave.status === "none"
            ? t("currentSave.noSave")
            : t("currentSave.clubUnknown")
      : null;

  return (
    <header className="topnav">
      <div className="topnav__left">
        <div className="topnav__crumbs">
          <span className="topnav__crumb">{t("app.name")}</span>
          <span className="topnav__sep">/</span>
          <span className="topnav__crumb is-current">{title}</span>
        </div>
        <h1 className="topnav__title">
          {club && active === "dashboard" ? club : title}
        </h1>
        {subtitle ? <p className="topnav__subtitle">{subtitle}</p> : null}
      </div>

      <div className="topnav__right">
        <SaveStatusBadge />

        <label className="topnav__search">
          <Icon name="search" size={16} />
          <input
            type="search"
            placeholder={t("topnav.searchPlaceholder")}
            aria-label={t("topnav.searchAria")}
          />
          <kbd className="topnav__kbd">
            <Icon name="command" size={11} />K
          </kbd>
        </label>

        <CloudSyncIndicator />

        <button
          type="button"
          className="topnav__icon-btn"
          aria-label={t("topnav.notifications")}
        >
          <Icon name="bell" size={17} />
          <span className="topnav__notify" />
        </button>

        <Button variant="primary" size="sm">
          <Icon name="sparkles" size={14} />
          {t("topnav.askScoutiq")}
        </Button>

        <UserMenu onOpenProfile={onOpenProfile} />
      </div>
    </header>
  );
}
