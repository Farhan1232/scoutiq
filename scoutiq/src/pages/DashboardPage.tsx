import { useTranslation } from "react-i18next";
import {
  ACTIVITY_META,
  FIXTURE_META,
  PERFORMER_META,
  REC_META,
  STAT_META,
} from "../data/mock";
import { FormSpark } from "../components/dashboard/FormSpark";
import { CurrentClubIdentity } from "../components/save/CurrentClubIdentity";
import { Badge } from "../components/ui/Badge";
import { Button } from "../components/ui/Button";
import { Icon } from "../components/icons/Icon";
import { RecommendationCard } from "../components/ui/RecommendationCard";
import { StatCard } from "../components/ui/StatCard";
import { useCurrentSave, useSave } from "../save/SaveContext";
import "./DashboardPage.css";

function difficultyKey(level: number) {
  if (level >= 5) return "elite";
  if (level >= 4) return "hard";
  if (level >= 3) return "medium";
  return "soft";
}

export function DashboardPage() {
  const { t } = useTranslation();
  const currentSave = useCurrentSave();
  const { identityProgress } = useSave();

  return (
    <div className="dashboard">
      <section className="dashboard__save-identity" aria-label={t("currentSave.sectionAria")}>
        <CurrentClubIdentity
          context={currentSave}
          variant="dashboard"
          showSeason
          showFileMeta
          identityProgress={identityProgress}
        />
      </section>

      <section
        className="dashboard__stats"
        aria-label={t("dashboard.statsAria")}
      >
        {STAT_META.map((stat, index) => (
          <StatCard
            key={stat.id}
            index={index}
            trend={stat.trend}
            label={t(`dashboard.stats.${stat.id}.label`)}
            value={t(`dashboard.stats.${stat.id}.value`)}
            delta={t(`dashboard.stats.${stat.id}.delta`)}
            caption={t(`dashboard.stats.${stat.id}.caption`)}
          />
        ))}
      </section>

      <section className="dashboard__section">
        <div className="dashboard__section-head">
          <div>
            <h2>{t("dashboard.recommendations.title")}</h2>
            <p>{t("dashboard.recommendations.subtitle")}</p>
          </div>
          <Button variant="ghost" size="sm">
            {t("common.viewAll")}
            <Icon name="arrowUpRight" size={14} />
          </Button>
        </div>
        <div className="dashboard__recs">
          {REC_META.map((rec, index) => (
            <RecommendationCard
              key={rec.id}
              index={index}
              priority={rec.priority}
              title={t(`dashboard.recs.${rec.id}.title`)}
              description={t(`dashboard.recs.${rec.id}.description`)}
              category={t(`dashboard.recs.${rec.id}.category`)}
              impact={t(`dashboard.recs.${rec.id}.impact`)}
              meta={t(`dashboard.recs.${rec.id}.meta`)}
            />
          ))}
        </div>
      </section>

      <div className="dashboard__grid">
        <section className="panel">
          <div className="panel__head">
            <div>
              <h2>{t("dashboard.fixtures.title")}</h2>
              <p>{t("dashboard.fixtures.subtitle")}</p>
            </div>
            <Badge tone="neutral">
              <Icon name="calendar" size={12} />
              {t("dashboard.fixtures.nextFour")}
            </Badge>
          </div>
          <ul className="fixture-list">
            {FIXTURE_META.map((fixture) => (
              <li key={fixture.id} className="fixture-item">
                <div className="fixture-item__main">
                  <span className="fixture-item__venue">{fixture.venue}</span>
                  <div>
                    <p className="fixture-item__opponent">
                      {t(`dashboard.fixtureItems.${fixture.id}.opponent`)}
                    </p>
                    <p className="fixture-item__meta">
                      {t(`dashboard.fixtureItems.${fixture.id}.competition`)} ·{" "}
                      {t(`dashboard.fixtureItems.${fixture.id}.date`)}
                    </p>
                  </div>
                </div>
                <div className="fixture-item__side">
                  <div
                    className="fixture-item__bars"
                    aria-label={t("common.difficulty", {
                      level: fixture.difficulty,
                    })}
                  >
                    {Array.from({ length: 5 }).map((_, i) => (
                      <span
                        key={i}
                        className={
                          i < fixture.difficulty ? "is-on" : undefined
                        }
                      />
                    ))}
                  </div>
                  <span className="fixture-item__diff">
                    {t(
                      `dashboard.fixtures.difficulty.${difficultyKey(fixture.difficulty)}`,
                    )}
                  </span>
                </div>
              </li>
            ))}
          </ul>
        </section>

        <section className="panel">
          <div className="panel__head">
            <div>
              <h2>{t("dashboard.performers.title")}</h2>
              <p>{t("dashboard.performers.subtitle")}</p>
            </div>
            <Badge tone="accent">{t("dashboard.performers.inForm")}</Badge>
          </div>
          <ul className="performer-list">
            {PERFORMER_META.map((player) => (
              <li key={player.id} className="performer-item">
                <div className="performer-item__identity">
                  <span className="performer-item__pos">{player.position}</span>
                  <div>
                    <p className="performer-item__name">
                      {t(`dashboard.performerItems.${player.id}.name`)}
                    </p>
                    <p className="performer-item__role">
                      {t(`dashboard.performerItems.${player.id}.role`)}
                    </p>
                  </div>
                </div>
                <div className="performer-item__metrics">
                  <FormSpark values={player.form} />
                  <span className="performer-item__rating">
                    {player.rating.toFixed(1)}
                  </span>
                </div>
              </li>
            ))}
          </ul>
        </section>

        <section className="panel panel--wide">
          <div className="panel__head">
            <div>
              <h2>{t("dashboard.activity.title")}</h2>
              <p>{t("dashboard.activity.subtitle")}</p>
            </div>
            <Button variant="secondary" size="sm">
              <Icon name="target" size={14} />
              {t("dashboard.activity.openInbox")}
            </Button>
          </div>
          <ul className="activity-list">
            {ACTIVITY_META.map((item) => (
              <li key={item.id} className="activity-item">
                <span
                  className={`activity-item__tone activity-item__tone--${item.tone}`}
                />
                <div className="activity-item__body">
                  <p className="activity-item__title">
                    {t(`dashboard.activityItems.${item.id}.title`)}
                  </p>
                  <p className="activity-item__detail">
                    {t(`dashboard.activityItems.${item.id}.detail`)}
                  </p>
                </div>
                <time className="activity-item__time">
                  {t(`dashboard.activityItems.${item.id}.time`)}
                </time>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </div>
  );
}
