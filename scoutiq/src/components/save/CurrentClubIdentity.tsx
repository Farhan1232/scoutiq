import { useTranslation } from "react-i18next";
import {
  competitionIdWithoutName,
  displayClubName,
  displayCompetitionName,
  displaySeason,
  isPossibleConfidence,
  type CurrentSaveContext,
} from "../../save/currentSaveTypes";
import type { IdentityProgress } from "../../save/fmIdentityApi";
import { Icon } from "../icons/Icon";
import "./CurrentClubIdentity.css";

interface CurrentClubIdentityProps {
  context: CurrentSaveContext;
  variant?: "sidebar" | "dashboard" | "compact";
  showSeason?: boolean;
  showFileMeta?: boolean;
  identityProgress?: IdentityProgress | null;
}

export function CurrentClubIdentity({
  context,
  variant = "sidebar",
  showSeason = true,
  showFileMeta = false,
  identityProgress = null,
}: CurrentClubIdentityProps) {
  const { t } = useTranslation();
  const club = displayClubName(context);
  const competition = displayCompetitionName(context);
  // The league can resolve without a name: FM only stores a competition's name
  // when it differs from the installed database's default.
  const competitionId = competitionIdWithoutName(context);
  const season = displaySeason(context);
  const clubPossible =
    club && isPossibleConfidence(context.confidence.club);
  const compPossible =
    competition && isPossibleConfidence(context.confidence.competition);
  const scanningDb =
    !!identityProgress && identityProgress.stage !== "done";
  const progressPct = Math.max(
    0,
    Math.min(100, identityProgress?.progress ?? 0),
  );

  if (context.status === "none") {
    return (
      <div className={`club-identity club-identity--${variant}`}>
        {variant === "dashboard" ? (
          <p className="club-identity__club">{t("app.name")}</p>
        ) : null}
        <p className="club-identity__line">{t("currentSave.noSave")}</p>
      </div>
    );
  }

  if (context.status === "loading") {
    return (
      <div className={`club-identity club-identity--${variant}`}>
        <div className="club-identity__row">
          <span className="club-identity__icon" aria-hidden>
            <Icon name="analytics" size={variant === "sidebar" ? 14 : 18} />
          </span>
          <div className="club-identity__body">
            <p className="club-identity__club">{t("currentSave.loading")}</p>
            <p className="club-identity__line">{t("currentSave.detectingClub")}</p>
            {scanningDb ? (
              <IdentityProgressBar
                progress={identityProgress}
                progressPct={progressPct}
                variant={variant}
              />
            ) : null}
          </div>
        </div>
      </div>
    );
  }

  if (context.status === "error") {
    return (
      <div className={`club-identity club-identity--${variant}`}>
        <p className="club-identity__club">{t("currentSave.clubUnknown")}</p>
        <p className="club-identity__line muted">
          {context.errorMessage ?? t("currentSave.competitionUnknown")}
        </p>
      </div>
    );
  }

  const clubLabel = club
    ? clubPossible
      ? t("currentSave.possibly", { value: club })
      : club
    : context.status === "partial"
      ? t("currentSave.detectingClub")
      : t("currentSave.clubUnknown");

  const competitionLabel = competition
    ? compPossible
      ? t("currentSave.possibly", { value: competition })
      : competition
    : competitionId
      ? t("currentSave.competitionById", { id: competitionId })
      : club || scanningDb
        ? t("currentSave.detectingCompetition")
        : t("currentSave.competitionUnknown");

  return (
    <div className={`club-identity club-identity--${variant}`}>
      <div className="club-identity__row">
        <span className="club-identity__icon" aria-hidden>
          <Icon name="squad" size={variant === "sidebar" ? 14 : 18} />
        </span>
        <div className="club-identity__body">
          <p className="club-identity__club">{clubLabel}</p>
          <p className="club-identity__line">{competitionLabel}</p>
          {scanningDb && !competition ? (
            <IdentityProgressBar
              progress={identityProgress}
              progressPct={progressPct}
              variant={variant}
            />
          ) : null}
          {showSeason && season ? (
            <p className="club-identity__season">
              {t("currentSave.seasonLine", { season })}
            </p>
          ) : null}
          {showFileMeta && context.fileName ? (
            <p className="club-identity__meta">
              {t("currentSave.saveFile", { file: context.fileName })}
              {context.inGameDate &&
              (context.confidence.date === "confirmed" ||
                context.confidence.date === "high" ||
                context.confidence.date === "medium")
                ? ` · ${t("currentSave.inGameDate", { date: context.inGameDate })}`
                : null}
            </p>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function IdentityProgressBar({
  progress,
  progressPct,
  variant,
}: {
  progress: IdentityProgress | null;
  progressPct: number;
  variant: "sidebar" | "dashboard" | "compact";
}) {
  const { t } = useTranslation();
  if (!progress) return null;
  const label =
    progress.framesTotal > 0
      ? t("currentSave.identityProgress", {
          progress: progressPct,
          scanned: progress.framesScanned,
          total: progress.framesTotal,
        })
      : progress.message || t("currentSave.readingCompetitionDb");

  return (
    <div
      className={`club-identity__progress club-identity__progress--${variant}`}
      role="progressbar"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={progressPct}
      aria-label={label}
    >
      <div className="club-identity__progress-track">
        <div
          className="club-identity__progress-fill"
          style={{ width: `${progressPct}%` }}
        />
      </div>
      {variant === "dashboard" ? (
        <p className="club-identity__progress-label">{label}</p>
      ) : null}
    </div>
  );
}
