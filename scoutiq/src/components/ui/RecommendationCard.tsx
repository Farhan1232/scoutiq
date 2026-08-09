import { useTranslation } from "react-i18next";
import type { RecommendationPriority } from "../../types";
import { Badge } from "./Badge";
import { Icon } from "../icons/Icon";
import "./RecommendationCard.css";

interface RecommendationCardProps {
  title: string;
  description: string;
  category: string;
  priority: RecommendationPriority;
  impact: string;
  meta: string;
  index?: number;
}

const priorityTone = {
  high: "danger",
  medium: "warning",
  low: "neutral",
} as const;

export function RecommendationCard({
  title,
  description,
  category,
  priority,
  impact,
  meta,
  index = 0,
}: RecommendationCardProps) {
  const { t } = useTranslation();

  return (
    <article
      className="rec-card"
      style={{ animationDelay: `${120 + index * 60}ms` }}
    >
      <div className="rec-card__top">
        <Badge tone="accent">{category}</Badge>
        <Badge tone={priorityTone[priority]}>
          {t(`common.priority.${priority}`)}
        </Badge>
      </div>

      <h3 className="rec-card__title">{title}</h3>
      <p className="rec-card__desc">{description}</p>

      <div className="rec-card__footer">
        <span className="rec-card__impact">
          <Icon name="sparkles" size={14} />
          {impact}
        </span>
        <span className="rec-card__meta">{meta}</span>
      </div>
    </article>
  );
}
