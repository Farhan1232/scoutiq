import { useTranslation } from "react-i18next";
import { Icon } from "../components/icons/Icon";
import type { IconName, NavId } from "../types";
import "./PlaceholderPage.css";

interface PlaceholderPageProps {
  id: Exclude<NavId, "dashboard" | "settings" | "squad" | "saveAnalyse">;
  title: string;
}

const pageIcons: Record<PlaceholderPageProps["id"], IconName> = {
  transfers: "transfers",
  tactics: "tactics",
  analytics: "analytics",
  finances: "finances",
  scouting: "scouting",
};

export function PlaceholderPage({ id, title }: PlaceholderPageProps) {
  const { t } = useTranslation();

  return (
    <div className="placeholder">
      <div className="placeholder__card">
        <div className="placeholder__icon">
          <Icon name={pageIcons[id]} size={22} />
        </div>
        <h2>{title}</h2>
        <p>{t(`pages.${id}.blurb`)}</p>
      </div>
    </div>
  );
}
