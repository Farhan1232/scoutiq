import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../ui/Button";
import { probeFmValue } from "../../save/analyzeFmSave";
import type {
  ForensicReport,
  ProbeQueryType,
  ProbeResult,
} from "../../save/fmForensic";

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

interface ForensicPanelProps {
  forensic: ForensicReport;
  savePath: string;
}

export function ForensicPanel({ forensic, savePath }: ForensicPanelProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState("");
  const [encodingFilter, setEncodingFilter] = useState("all");
  const [sourceFilter, setSourceFilter] = useState("all");
  const [probeType, setProbeType] = useState<ProbeQueryType>("club");
  const [probeValue, setProbeValue] = useState("");
  const [probeBusy, setProbeBusy] = useState(false);
  const [probeResult, setProbeResult] = useState<ProbeResult | null>(null);
  const [probeError, setProbeError] = useState<string | null>(null);

  const filteredStrings = useMemo(() => {
    const q = filter.trim().toLowerCase();
    return forensic.strings.filter((s) => {
      if (encodingFilter !== "all" && s.encoding !== encodingFilter) return false;
      if (sourceFilter !== "all" && s.source !== sourceFilter) return false;
      if (!q) return true;
      return (
        s.value.toLowerCase().includes(q) ||
        String(s.offset).includes(q) ||
        s.encoding.toLowerCase().includes(q)
      );
    });
  }, [forensic.strings, filter, encodingFilter, sourceFilter]);

  const visibleStrings = filteredStrings.slice(0, 80);

  async function runProbe() {
    setProbeBusy(true);
    setProbeError(null);
    const result = await probeFmValue(savePath, probeValue, probeType);
    setProbeBusy(false);
    if (!result.ok) {
      setProbeResult(null);
      setProbeError(result.error.detail ?? t("saveAnalyse.forensic.probeFailed"));
      return;
    }
    setProbeResult(result.result);
  }

  const log = forensic.log;

  return (
    <section className="save-analyse__panel save-analyse__panel--wide save-analyse__forensic">
      <h3>{t("saveAnalyse.forensic.title")}</h3>
      <p className="save-analyse__muted">{t("saveAnalyse.forensic.subtitle")}</p>

      <div className="save-analyse__forensic-log">
        <h4>{t("saveAnalyse.forensic.logTitle")}</h4>
        <ul>
          {log.summaryLines.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
        <dl className="save-analyse__meta-grid">
          <div>
            <dt>{t("saveAnalyse.forensic.encAscii")}</dt>
            <dd>{log.stringsByEncoding.ascii}</dd>
          </div>
          <div>
            <dt>{t("saveAnalyse.forensic.encUtf8")}</dt>
            <dd>{log.stringsByEncoding.utf8}</dd>
          </div>
          <div>
            <dt>{t("saveAnalyse.forensic.encUtf16Le")}</dt>
            <dd>{log.stringsByEncoding.utf16Le}</dd>
          </div>
          <div>
            <dt>{t("saveAnalyse.forensic.encUtf16Be")}</dt>
            <dd>{log.stringsByEncoding.utf16Be}</dd>
          </div>
          <div>
            <dt>{t("saveAnalyse.forensic.zstdOk")}</dt>
            <dd>
              {log.decompressedBlocksProcessed} / {log.compressedBlocksDetected}
            </dd>
          </div>
          <div>
            <dt>{t("saveAnalyse.forensic.zstdBytes")}</dt>
            <dd>{formatBytes(log.decompressedBytesTotal)}</dd>
          </div>
        </dl>
        {log.promisingOffsets.length > 0 ? (
          <p>
            <strong>{t("saveAnalyse.forensic.promising")}:</strong>{" "}
            {log.promisingOffsets.join(", ")}
          </p>
        ) : null}
      </div>

      <div className="save-analyse__probe">
        <h4>{t("saveAnalyse.forensic.probeTitle")}</h4>
        <div className="save-analyse__probe-row">
          <label>
            {t("saveAnalyse.forensic.probeType")}
            <select
              value={probeType}
              onChange={(e) => setProbeType(e.target.value as ProbeQueryType)}
            >
              <option value="manager">{t("saveAnalyse.metadata.manager")}</option>
              <option value="club">{t("saveAnalyse.metadata.club")}</option>
              <option value="competition">
                {t("saveAnalyse.metadata.competition")}
              </option>
              <option value="free">{t("saveAnalyse.forensic.probeFree")}</option>
            </select>
          </label>
          <label className="save-analyse__probe-value">
            {t("saveAnalyse.forensic.probeValue")}
            <input
              value={probeValue}
              onChange={(e) => setProbeValue(e.target.value)}
              placeholder='z. B. TSV 1860 München'
            />
          </label>
          <Button
            type="button"
            variant="secondary"
            disabled={probeBusy || !probeValue.trim()}
            onClick={() => void runProbe()}
          >
            {probeBusy
              ? t("saveAnalyse.forensic.probeRunning")
              : t("saveAnalyse.forensic.probeRun")}
          </Button>
        </div>
        {probeError ? <p className="save-analyse__error-text">{probeError}</p> : null}
        {probeResult ? (
          <div className="save-analyse__probe-result">
            <p>
              {t("saveAnalyse.forensic.probeHits", {
                count: probeResult.matches.length,
              })}
              {probeResult.encodingsHit.length > 0
                ? ` · ${probeResult.encodingsHit.join(", ")}`
                : ""}
            </p>
            {probeResult.notes.map((n) => (
              <p key={n} className="save-analyse__muted">
                {n}
              </p>
            ))}
            {probeResult.matches.map((m) => (
              <details key={`${m.offset}-${m.encoding}-${m.source}`} open={false}>
                <summary>
                  @{m.offset} · {m.encoding} · {m.source}
                </summary>
                <pre className="save-analyse__hex">{m.contextHex}</pre>
                {m.nearbyStrings.length > 0 ? (
                  <p>
                    {t("saveAnalyse.forensic.nearbyStrings")}:{" "}
                    {m.nearbyStrings.join(" · ")}
                  </p>
                ) : null}
                {m.nearbyIntegers.length > 0 ? (
                  <ul className="save-analyse__int-list">
                    {m.nearbyIntegers.slice(0, 16).map((i) => (
                      <li key={`${i.offset}-${i.width}-${i.endian}-${i.signed}-${i.value}`}>
                        @{i.offset} u{i.width}/{i.endian}
                        {i.signed ? "s" : ""} = {i.value}
                      </li>
                    ))}
                  </ul>
                ) : null}
              </details>
            ))}
          </div>
        ) : null}
      </div>

      <div className="save-analyse__string-catalog">
        <h4>{t("saveAnalyse.forensic.catalogTitle")}</h4>
        <div className="save-analyse__probe-row">
          <input
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder={t("saveAnalyse.forensic.filterPlaceholder")}
          />
          <select
            value={encodingFilter}
            onChange={(e) => setEncodingFilter(e.target.value)}
          >
            <option value="all">{t("saveAnalyse.forensic.allEncodings")}</option>
            <option value="ascii">ASCII</option>
            <option value="utf8">UTF-8</option>
            <option value="utf16le">UTF-16 LE</option>
            <option value="utf16be">UTF-16 BE</option>
          </select>
          <select
            value={sourceFilter}
            onChange={(e) => setSourceFilter(e.target.value)}
          >
            <option value="all">{t("saveAnalyse.forensic.allSources")}</option>
            <option value="raw">raw</option>
            <option value="zstd">zstd</option>
          </select>
        </div>
        <p className="save-analyse__muted">
          {t("saveAnalyse.forensic.catalogCount", {
            shown: visibleStrings.length,
            total: filteredStrings.length,
          })}
        </p>
        <div className="save-analyse__string-list">
          {visibleStrings.map((s) => (
            <details key={`${s.offset}-${s.encoding}-${s.source}-${s.value}`}>
              <summary>
                @{s.offset} · {s.encoding}/{s.source} · {s.value} ({s.length})
              </summary>
              <p>
                <strong>{t("saveAnalyse.forensic.before")}:</strong>
              </p>
              <pre className="save-analyse__hex">{s.contextBeforeHex || "—"}</pre>
              <p>
                <strong>{t("saveAnalyse.forensic.after")}:</strong>
              </p>
              <pre className="save-analyse__hex">{s.contextAfterHex || "—"}</pre>
            </details>
          ))}
        </div>
      </div>

      {forensic.repeatedPatterns.length > 0 ? (
        <div>
          <h4>{t("saveAnalyse.forensic.patternsTitle")}</h4>
          <ul>
            {forensic.repeatedPatterns.slice(0, 20).map((g) => (
              <li key={g.value}>
                „{g.value}“ ×{g.occurrences} @ {g.offsets.slice(0, 6).join(", ")}
                {g.offsets.length > 6 ? "…" : ""} — {g.note}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {forensic.compressedBlocks.length > 0 ? (
        <div>
          <h4>{t("saveAnalyse.forensic.blocksTitle")}</h4>
          <ul>
            {forensic.compressedBlocks.map((b) => (
              <li key={`${b.offset}-${b.note}`}>
                @{b.offset} {b.magic}{" "}
                {b.success
                  ? `✓ ${formatBytes(b.decompressedBytes)}`
                  : "✗"}{" "}
                — {b.note}
              </li>
            ))}
          </ul>
        </div>
      ) : null}

      {forensic.entropyMap.length > 0 ? (
        <div>
          <h4>{t("saveAnalyse.forensic.entropyTitle")}</h4>
          <div className="save-analyse__entropy">
            {forensic.entropyMap.map((b) => (
              <div
                key={b.offset}
                className={`save-analyse__entropy-block save-analyse__entropy-block--${b.kind === "textartig" ? "low" : b.kind === "komprimiert/hoch" ? "high" : "mid"}`}
                title={`@${b.offset} H=${b.entropy.toFixed(2)} ${b.kind}`}
              />
            ))}
          </div>
        </div>
      ) : null}
    </section>
  );
}
