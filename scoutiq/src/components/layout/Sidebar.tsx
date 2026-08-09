import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { NAV_ITEMS } from "../../data/mock";
import {
  isDeveloperMode,
  subscribeDeveloperMode,
} from "../../dev/developerMode";
import { useCurrentSave, useSave } from "../../save/SaveContext";
import type { NavId } from "../../types";
import { Icon } from "../icons/Icon";
import { CurrentClubIdentity } from "../save/CurrentClubIdentity";
import "./Sidebar.css";

interface SidebarProps {
  active: NavId;
  onNavigate: (id: NavId) => void;
}

export function Sidebar({ active, onNavigate }: SidebarProps) {
  const { t } = useTranslation();
  const currentSave = useCurrentSave();
  const { identityProgress } = useSave();
  const [devMode, setDevMode] = useState(isDeveloperMode);

  useEffect(() => subscribeDeveloperMode(setDevMode), []);

  const items = NAV_ITEMS.filter(
    (item) => item.id !== "saveAnalyse" || devMode,
  );

  return (
    <aside className="sidebar">
      <div className="sidebar__brand">
        <div className="sidebar__logo" aria-hidden="true">
          <span className="sidebar__logo-mark" />
        </div>
        <div className="sidebar__brand-text">
          <span className="sidebar__product">{t("app.name")}</span>
          <CurrentClubIdentity
            context={currentSave}
            variant="sidebar"
            identityProgress={identityProgress}
          />
        </div>
      </div>

      <nav className="sidebar__nav" aria-label={t("nav.primary")}>
        <p className="sidebar__section">{t("nav.workspace")}</p>
        <ul className="sidebar__list">
          {items.map((item) => {
            const isActive = item.id === active;
            return (
              <li key={item.id}>
                <button
                  type="button"
                  className={`sidebar__link${isActive ? " is-active" : ""}`}
                  onClick={() => onNavigate(item.id)}
                  aria-current={isActive ? "page" : undefined}
                  title={t(`nav.${item.id}`)}
                  aria-label={t(`nav.${item.id}`)}
                >
                  <Icon name={item.icon} size={17} />
                  <span>{t(`nav.${item.id}`)}</span>
                  {item.badge ? (
                    <span className="sidebar__badge">{item.badge}</span>
                  ) : null}
                </button>
              </li>
            );
          })}
        </ul>
      </nav>

      <div className="sidebar__footer">
        <CurrentClubIdentity
          context={currentSave}
          variant="compact"
          showSeason
          identityProgress={identityProgress}
        />
      </div>
    </aside>
  );
}
