import type { ReactNode } from "react";
import "./Badge.css";

type BadgeTone = "accent" | "neutral" | "warning" | "danger" | "info";

interface BadgeProps {
  children: ReactNode;
  tone?: BadgeTone;
  size?: "sm" | "md";
}

export function Badge({ children, tone = "neutral", size = "sm" }: BadgeProps) {
  return (
    <span className={`badge badge--${tone} badge--${size}`}>{children}</span>
  );
}
