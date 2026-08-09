import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useAuth } from "../../auth";
import { Icon } from "../icons/Icon";
import "./UserMenu.css";

interface UserMenuProps {
  onOpenProfile: () => void;
}

export function UserMenu({ onOpenProfile }: UserMenuProps) {
  const { t } = useTranslation();
  const { user, signOut } = useAuth();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const menuId = useId();

  const firstName = user?.displayName.split(/\s+/)[0] ?? t("club.managerShort");

  useEffect(() => {
    if (!open) return;

    const onPointerDown = (event: MouseEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };

    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  return (
    <div className={`user-menu${open ? " is-open" : ""}`} ref={rootRef}>
      <button
        type="button"
        className="topnav__profile"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-controls={menuId}
        onClick={() => setOpen((value) => !value)}
      >
        <span className="topnav__avatar">{user?.initials ?? "SQ"}</span>
        <span className="topnav__profile-meta">
          <span className="topnav__name">{firstName}</span>
          <span className="topnav__role">{t("topnav.role")}</span>
        </span>
        <Icon name="chevronDown" size={14} />
      </button>

      {open ? (
        <div className="user-menu__dropdown" id={menuId} role="menu">
          <div className="user-menu__header">
            <span className="user-menu__avatar">{user?.initials ?? "SQ"}</span>
            <div className="user-menu__identity">
              <strong>{user?.displayName ?? t("app.name")}</strong>
              <span>{user?.email}</span>
            </div>
          </div>

          <button
            type="button"
            className="user-menu__item"
            role="menuitem"
            onClick={() => {
              setOpen(false);
              onOpenProfile();
            }}
          >
            <Icon name="user" size={15} />
            {t("topnav.menu.profile")}
          </button>

          <button
            type="button"
            className="user-menu__item user-menu__item--danger"
            role="menuitem"
            onClick={() => {
              setOpen(false);
              void signOut();
            }}
          >
            <Icon name="logOut" size={15} />
            {t("topnav.menu.signOut")}
          </button>
        </div>
      ) : null}
    </div>
  );
}
