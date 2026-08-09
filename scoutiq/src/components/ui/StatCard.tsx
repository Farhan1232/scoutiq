import type { TrendDirection } from "../../types";
import { Icon } from "../icons/Icon";
import "./StatCard.css";

interface StatCardProps {
  label: string;
  value: string;
  delta: string;
  caption: string;
  trend: TrendDirection;
  index?: number;
}

export function StatCard({
  label,
  value,
  delta,
  caption,
  trend,
  index = 0,
}: StatCardProps) {
  const trendIcon =
    trend === "up"
      ? "arrowUpRight"
      : trend === "down"
        ? "arrowDownRight"
        : "minus";

  return (
    <article
      className="stat-card"
      style={{ animationDelay: `${index * 50}ms` }}
    >
      <div className="stat-card__header">
        <span className="stat-card__label">{label}</span>
        <span className={`stat-card__delta stat-card__delta--${trend}`}>
          <Icon name={trendIcon} size={14} />
          {delta}
        </span>
      </div>
      <div className="stat-card__value">{value}</div>
      <p className="stat-card__caption">{caption}</p>
    </article>
  );
}
