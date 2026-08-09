import { useTranslation } from "react-i18next";
import { useSave } from "../../save";
import { Icon } from "../icons/Icon";
import "./SaveStatusBadge.css";

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  const mb = bytes / (1024 * 1024);
  if (mb >= 1) return `${mb.toFixed(1)} MB`;
  const kb = bytes / 1024;
  return `${kb.toFixed(0)} KB`;
}

export function SaveStatusBadge() {
  const { t } = useTranslation();
  const { fmSave, loadState, lastError, openFmSave } = useSave();

  const label =
    loadState === "loading"
      ? t("save.state.loading")
      : loadState === "error"
        ? t("save.state.error")
        : loadState === "loaded" && fmSave
          ? fmSave.fileName
          : t("save.state.none");

  const title =
    loadState === "loaded" && fmSave
      ? t("save.badgeTitleLoaded", {
          file: fmSave.fileName,
          size: formatBytes(fmSave.fileSizeBytes),
          modified: new Date(fmSave.lastModifiedAt).toLocaleString(),
        })
      : loadState === "error" && lastError
        ? t(lastError.messageKey)
        : t("save.badgeTitleIdle");

  return (
    <button
      type="button"
      className={`save-badge save-badge--${loadState}`}
      onClick={() => {
        void openFmSave();
      }}
      title={title}
      aria-label={title}
    >
      <Icon
        name={
          loadState === "error"
            ? "cloudOff"
            : loadState === "loaded"
              ? "squad"
              : "upload"
        }
        size={14}
      />
      <span className="save-badge__label">{label}</span>
      {loadState === "loaded" && fmSave ? (
        <span className="save-badge__meta">{formatBytes(fmSave.fileSizeBytes)}</span>
      ) : null}
    </button>
  );
}
