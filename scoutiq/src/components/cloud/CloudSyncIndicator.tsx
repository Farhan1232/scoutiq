import { useTranslation } from "react-i18next";
import { useCloudSync, type SyncStatus } from "../../cloud";
import { Icon } from "../icons/Icon";
import "./CloudSyncIndicator.css";

function statusIcon(status: SyncStatus): "cloud" | "cloudOff" {
  return status === "offline" || status === "error" || status === "unsigned"
    ? "cloudOff"
    : "cloud";
}

export function CloudSyncIndicator() {
  const { t } = useTranslation();
  const { status, isSyncing, syncNow, lastSyncedAt } = useCloudSync();

  const label = t(`cloud.statusLabel.${status}`);
  const title = lastSyncedAt
    ? t("cloud.indicatorTitleWithTime", {
        status: label,
        time: new Date(lastSyncedAt).toLocaleString(),
      })
    : t("cloud.indicatorTitle", { status: label });

  return (
    <button
      type="button"
      className={`cloud-indicator cloud-indicator--${status}`}
      onClick={() => {
        void syncNow();
      }}
      disabled={isSyncing}
      title={title}
      aria-label={title}
    >
      <Icon name={statusIcon(status)} size={15} />
      <span className="cloud-indicator__label">{label}</span>
      <span className="cloud-indicator__dot" aria-hidden="true" />
    </button>
  );
}
