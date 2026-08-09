import { useTranslation } from "react-i18next";
import { SYNC_DOMAINS, useCloudSync, type DomainSyncResult } from "../../cloud";
import { Badge } from "../ui/Badge";
import { Button } from "../ui/Button";
import { Icon } from "../icons/Icon";
import { SupabaseConnectionTest } from "./SupabaseConnectionTest";
import "./CloudSection.css";

function badgeTone(mode: "cloud" | "offline" | "local") {
  if (mode === "cloud") return "accent" as const;
  if (mode === "offline") return "warning" as const;
  return "neutral" as const;
}

function domainStatusLabelKey(result: DomainSyncResult): string {
  if (result.messageKey === "cloud.status.schemaMissing") {
    return "cloud.statusLabel.schemaMissing";
  }
  if (result.messageKey === "cloud.status.rlsDenied") {
    return "cloud.statusLabel.rlsDenied";
  }
  return `cloud.statusLabel.${result.status}`;
}

export function CloudSection() {
  const { t } = useTranslation();
  const { status, isSyncing, lastSyncedAt, lastResult, syncNow, modeBadge } =
    useCloudSync();

  const hasSchemaMissing = lastResult?.domains.some(
    (item) => item.messageKey === "cloud.status.schemaMissing",
  );

  return (
    <div className="cloud-section">
      <SupabaseConnectionTest />

      <div className="cloud-section__status">
        <div className="cloud-section__status-main">
          <span className={`cloud-section__icon cloud-section__icon--${status}`}>
            <Icon
              name={
                status === "offline" ||
                status === "error" ||
                status === "unsigned"
                  ? "cloudOff"
                  : "cloud"
              }
              size={18}
            />
          </span>
          <div>
            <p className="cloud-section__status-label">
              {t(`cloud.statusLabel.${status}`)}
            </p>
            <p className="cloud-section__status-meta">
              {lastSyncedAt
                ? t("cloud.lastSynced", {
                    time: new Date(lastSyncedAt).toLocaleString(),
                  })
                : t("cloud.neverSynced")}
            </p>
          </div>
        </div>
        <Badge tone={badgeTone(modeBadge)}>
          {t(`cloud.mode.${modeBadge}`)}
        </Badge>
      </div>

      <p className="cloud-section__copy">{t("cloud.description")}</p>

      {hasSchemaMissing ? (
        <p className="cloud-section__hint" role="status">
          {t("cloud.status.schemaMissing")}
        </p>
      ) : null}

      <ul className="cloud-section__domains">
        {SYNC_DOMAINS.map((domain) => {
          const domainResult = lastResult?.domains.find(
            (item) => item.domain === domain,
          );
          return (
            <li key={domain} className="cloud-section__domain">
              <span>{t(`cloud.domains.${domain}`)}</span>
              <strong>
                {domainResult
                  ? t(domainStatusLabelKey(domainResult))
                  : t("cloud.pending")}
              </strong>
            </li>
          );
        })}
      </ul>

      <div className="cloud-section__actions">
        <Button
          variant="secondary"
          size="sm"
          disabled={isSyncing || status === "unsigned"}
          onClick={() => {
            void syncNow();
          }}
        >
          <Icon name="cloud" size={14} />
          {isSyncing ? t("cloud.syncing") : t("cloud.syncNow")}
        </Button>
      </div>
    </div>
  );
}
