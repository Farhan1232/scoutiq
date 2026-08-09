import type { ReactNode } from "react";
import type { NavId } from "../../types";
import { Sidebar } from "./Sidebar";
import { TopNav } from "./TopNav";
import "./AppShell.css";

interface AppShellProps {
  active: NavId;
  title: string;
  onNavigate: (id: NavId) => void;
  onOpenProfile: () => void;
  children: ReactNode;
}

export function AppShell({
  active,
  title,
  onNavigate,
  onOpenProfile,
  children,
}: AppShellProps) {
  return (
    <div className="app-shell">
      <Sidebar active={active} onNavigate={onNavigate} />
      <div className="app-shell__main">
        <TopNav
          active={active}
          title={title}
          onOpenProfile={onOpenProfile}
        />
        <main className="app-shell__content">{children}</main>
      </div>
    </div>
  );
}
