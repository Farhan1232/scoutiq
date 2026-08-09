import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/Button";
import { Icon } from "../components/icons/Icon";
import { useSave } from "../save";
import {
  cancelFmAnalysis,
  exportFmDiagnostic,
  listenEngineDone,
  listenEngineProgress,
  parseFmContainer,
  probeFmContainer,
  startFmEngineAnalysis,
} from "../save/fmContainerApi";
import type {
  EngineProgress,
  FmEngineResult,
  KnownValueProbeResult,
  ProbeType,
} from "../save/fmContainer";
import type { CurrentSaveEntity } from "../save/currentSaveTypes";
import "./SaveAnalysePage.css";

function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
  }
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

function containerLabel(c: string | null | undefined): string {
  if (c === "fmf") return "FMF";
  if (c === "unknown") return "Unknown";
  return "—";
}

/** Show id only when known; never prefix a real name with "unknown". */
function formatEntityRecord(
  entity: CurrentSaveEntity,
  t: (key: string) => string,
): string {
  const name = entity.name?.trim() || entity.shortName?.trim() || null;
  if (entity.id && name) return `${entity.id} · ${name}`;
  if (name) return name;
  if (entity.id) return entity.id;
  return t("currentSave.unknownId");
}

export function SaveAnalysePage() {
  const { t } = useTranslation();
  const { fmSave, openFmSave, lastError, clearError, currentSave, identityProgress } =
    useSave();

  const [engine, setEngine] = useState<FmEngineResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [progress, setProgress] = useState<EngineProgress | null>(null);

  const [probeType, setProbeType] = useState<ProbeType>("free");
  const [probeValue, setProbeValue] = useState("");
  const [probeResult, setProbeResult] = useState<KnownValueProbeResult | null>(
    null,
  );

  const activePath = engine?.file.filePath ?? fmSave?.path ?? null;

  useEffect(() => {
    let unProgress: (() => void) | undefined;
    let unDone: (() => void) | undefined;
    void (async () => {
      unProgress = await listenEngineProgress((p) => setProgress(p));
      unDone = await listenEngineDone((payload) => {
        setBusy(false);
        setProgress(null);
        if (payload.ok && payload.result) {
          setEngine(payload.result);
          setError(null);
        } else {
          setError(payload.error ?? t("fmParser.unknownError"));
        }
      });
    })();
    return () => {
      unProgress?.();
      unDone?.();
    };
  }, [t]);

  const runParse = useCallback(async (path: string) => {
    setBusy(true);
    setError(null);
    setProbeResult(null);
    setProgress(null);
    try {
      await startFmEngineAnalysis(path);
    } catch {
      // Fallback without progress events.
      try {
        const result = await parseFmContainer(path);
        setEngine(result);
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        setEngine(null);
      } finally {
        setBusy(false);
      }
    }
  }, []);

  const onOpenAndParse = useCallback(async () => {
    clearError();
    await openFmSave();
  }, [clearError, openFmSave]);

  useEffect(() => {
    if (!fmSave?.path) return;
    if (engine?.file.filePath === fmSave.path) return;
    void runParse(fmSave.path);
  }, [fmSave?.path, engine?.file.filePath, runParse]);

  async function onCancel() {
    await cancelFmAnalysis();
    setBusy(false);
    setProgress(null);
  }

  async function onProbe() {
    if (!activePath || !probeValue.trim()) return;
    setBusy(true);
    setError(null);
    try {
      setProbeResult(await probeFmContainer(activePath, probeValue, probeType));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onExport() {
    if (!activePath) return;
    setBusy(true);
    try {
      const exp = await exportFmDiagnostic(
        activePath,
        probeValue || undefined,
        probeType,
      );
      const blob = new Blob([JSON.stringify(exp, null, 2)], {
        type: "application/json",
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `scoutiq-fm-diagnostic-${Date.now()}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="save-analyse">
      <header className="save-analyse__header">
        <div>
          <p className="save-analyse__eyebrow">{t("fmParser.eyebrow")}</p>
          <h1>{t("fmParser.title")}</h1>
          <p className="save-analyse__subtitle">{t("fmParser.subtitle")}</p>
          <p className="save-analyse__status-pill">{t("fmParser.status")}</p>
        </div>
        <div className="save-analyse__actions">
          {busy ? (
            <Button type="button" variant="secondary" onClick={() => void onCancel()}>
              {t("fmParser.cancel")}
            </Button>
          ) : null}
          <Button
            type="button"
            variant="secondary"
            disabled={busy || !activePath}
            onClick={() => activePath && void runParse(activePath)}
          >
            {t("fmParser.reparse")}
          </Button>
          <Button
            type="button"
            variant="primary"
            disabled={busy}
            onClick={() => void onOpenAndParse()}
          >
            <Icon name="upload" size={16} />
            {busy ? t("fmParser.busy") : t("fmParser.open")}
          </Button>
        </div>
      </header>

      {(error || lastError) && (
        <div className="save-analyse__error" role="alert">
          <p>{error ?? lastError?.detail ?? t("save.errors.inspectFailed")}</p>
          <button
            type="button"
            onClick={() => {
              setError(null);
              clearError();
            }}
          >
            {t("saveAnalyse.dismissError")}
          </button>
        </div>
      )}

      {busy && progress ? (
        <section className="save-analyse__panel save-analyse__panel--wide">
          <h3>{progress.message}</h3>
          <p>
            {progress.progress}% · {t("fmParser.blockProgress", {
              current: progress.currentBlock,
              total: progress.totalBlocks,
            })}{" "}
            · {formatBytes(progress.bytesProcessed)} /{" "}
            {formatBytes(progress.totalBytes)}
          </p>
        </section>
      ) : null}

      {identityProgress && identityProgress.stage !== "done" ? (
        <section className="save-analyse__panel save-analyse__panel--wide">
          <h3>{identityProgress.message || t("currentSave.readingCompetitionDb")}</h3>
          <div
            className="save-analyse__identity-progress"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={identityProgress.progress}
          >
            <div className="save-analyse__identity-progress-track">
              <div
                className="save-analyse__identity-progress-fill"
                style={{
                  width: `${Math.max(0, Math.min(100, identityProgress.progress))}%`,
                }}
              />
            </div>
            <p className="save-analyse__progress-meta">
              {identityProgress.framesTotal > 0
                ? t("currentSave.identityProgress", {
                    progress: identityProgress.progress,
                    scanned: identityProgress.framesScanned,
                    total: identityProgress.framesTotal,
                  })
                : `${identityProgress.progress} %`}
            </p>
          </div>
        </section>
      ) : null}

      {!fmSave && !engine ? (
        <section className="save-analyse__empty">
          <h2>{t("fmParser.emptyTitle")}</h2>
          <p>{t("fmParser.emptyBody")}</p>
        </section>
      ) : (
        <div className="save-analyse__grid">
          {/* A. Spielstand */}
          <section className="save-analyse__panel save-analyse__panel--wide">
            <h3>{t("currentSave.devTitle")}</h3>
            <dl className="save-analyse__meta-grid">
              <div>
                <dt>{t("currentSave.devManager")}</dt>
                <dd>{formatEntityRecord(currentSave.manager, t)}</dd>
              </div>
              <div>
                <dt>{t("currentSave.devClub")}</dt>
                <dd>{formatEntityRecord(currentSave.club, t)}</dd>
              </div>
              <div>
                <dt>{t("currentSave.devCompetition")}</dt>
                <dd>
                  {formatEntityRecord(currentSave.competition, t)}
                  {" · "}
                  {t("currentSave.devNameSource", {
                    source: currentSave.competitionNameSource,
                  })}
                </dd>
              </div>
            </dl>
            {currentSave.evidence.length > 0 ? (
              <ul className="save-analyse__notes">
                <li>
                  <strong>{t("currentSave.devEvidence")}</strong>
                </li>
                {currentSave.evidence.slice(0, 12).map((e) => (
                  <li key={`${e.kind}-${e.detail}`}>
                    [{e.kind}] {e.detail}
                  </li>
                ))}
              </ul>
            ) : null}
          </section>

          <section className="save-analyse__panel save-analyse__panel--wide">
            <h3>{t("fmParser.saveTitle")}</h3>
            <dl className="save-analyse__meta-grid">
              <div>
                <dt>{t("fmParser.fileName")}</dt>
                <dd>{engine?.file.fileName ?? fmSave?.fileName ?? "—"}</dd>
              </div>
              <div>
                <dt>{t("fmParser.fileSize")}</dt>
                <dd>{formatBytes(engine?.file.fileSize ?? fmSave?.fileSizeBytes ?? 0)}</dd>
              </div>
              <div>
                <dt>{t("fmParser.containerFormat")}</dt>
                <dd>{containerLabel(engine?.file.detectedContainer)}</dd>
              </div>
              <div>
                <dt>{t("fmParser.analysisStatus")}</dt>
                <dd>{t("fmParser.calibratingField")}</dd>
              </div>
            </dl>
          </section>

          {/* B. Datenblöcke */}
          <section className="save-analyse__panel save-analyse__panel--wide">
            <h3>{t("fmParser.blocksTitle")}</h3>
            {engine ? (
              <dl className="save-analyse__meta-grid">
                <div>
                  <dt>{t("fmParser.blockCandidates")}</dt>
                  <dd>{engine.blockStats.candidates}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.blockValidated")}</dt>
                  <dd>{engine.blockStats.validated}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.blockDecompressed")}</dt>
                  <dd>{engine.blockStats.decompressed}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.blockErrors")}</dt>
                  <dd>{engine.blockStats.errors}</dd>
                </div>
              </dl>
            ) : (
              <p className="save-analyse__muted">{t("fmParser.waitingParse")}</p>
            )}
          </section>

          {/* C. Strukturen */}
          <section className="save-analyse__panel save-analyse__panel--wide">
            <h3>{t("fmParser.structuresTitle")}</h3>
            {engine ? (
              <dl className="save-analyse__meta-grid">
                <div>
                  <dt>{t("fmParser.stringTables")}</dt>
                  <dd>{engine.structureStats.stringTables}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.recordTables")}</dt>
                  <dd>{engine.structureStats.recordTables}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.indexTables")}</dt>
                  <dd>{engine.structureStats.indexTables}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.internalFiles")}</dt>
                  <dd>{engine.structureStats.internalFiles}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.unknownStructures")}</dt>
                  <dd>{engine.structureStats.unknown}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.stringsIndexed")}</dt>
                  <dd>{engine.stringEntries}</dd>
                </div>
              </dl>
            ) : null}
            {engine?.structures?.length ? (
              <ul className="save-analyse__notes">
                {engine.structures.slice(0, 24).map((s, i) => (
                  <li key={`${s.blockIndex}-${s.offset}-${i}`}>
                    #{s.blockIndex} @{s.offset} · {s.structureType} · {s.confidence}
                    {s.evidence[0] ? ` — ${s.evidence[0]}` : ""}
                  </li>
                ))}
              </ul>
            ) : null}
          </section>

          {/* D/E Probe */}
          <section className="save-analyse__panel save-analyse__panel--wide">
            <h3>{t("fmParser.probeTitle")}</h3>
            <div className="save-analyse__probe-row">
              <label>
                {t("fmParser.probeType")}
                <select
                  value={probeType}
                  onChange={(e) => setProbeType(e.target.value as ProbeType)}
                >
                  <option value="manager">{t("saveAnalyse.metadata.manager")}</option>
                  <option value="club">{t("saveAnalyse.metadata.club")}</option>
                  <option value="player">{t("fmParser.probePlayer")}</option>
                  <option value="competition">
                    {t("saveAnalyse.metadata.competition")}
                  </option>
                  <option value="free">{t("fmParser.probeFree")}</option>
                </select>
              </label>
              <label className="save-analyse__probe-value">
                {t("fmParser.probeValue")}
                <input
                  value={probeValue}
                  onChange={(e) => setProbeValue(e.target.value)}
                  placeholder={t("fmParser.probePlaceholder")}
                />
              </label>
              <Button
                type="button"
                variant="secondary"
                disabled={busy || !activePath || !probeValue.trim()}
                onClick={() => void onProbe()}
              >
                {t("fmParser.probeRun")}
              </Button>
            </div>
            {probeResult ? (
              <div className="save-analyse__probe-result">
                <p>
                  {t("fmParser.probeHits", { count: probeResult.matches.length })} ·{" "}
                  {t("fmParser.blocksSearched", {
                    count: probeResult.blocksSearched,
                  })}
                </p>
                {probeResult.notes.map((n) => (
                  <p key={n} className="save-analyse__muted">
                    {n}
                  </p>
                ))}
                <div className="save-analyse__match-table">
                  <div className="save-analyse__match-head">
                    <span>{t("fmParser.colValue")}</span>
                    <span>{t("fmParser.colBlock")}</span>
                    <span>{t("fmParser.colOffset")}</span>
                    <span>{t("fmParser.colEncoding")}</span>
                    <span>{t("fmParser.colContext")}</span>
                  </div>
                  {probeResult.matches.map((m) => (
                    <details
                      key={`${m.blockIndex}-${m.decompressedOffset}-${m.encoding}`}
                    >
                      <summary>
                        <span>{m.searchedValue}</span>
                        <span>{m.blockIndex}</span>
                        <span>
                          @{m.decompressedOffset} / file @{m.sourceFileOffset}
                        </span>
                        <span>{String(m.encoding)}</span>
                        <span>{t("fmParser.showContext")}</span>
                      </summary>
                      <pre className="save-analyse__hex">{m.contextBeforeHex}</pre>
                      <pre className="save-analyse__hex">{m.contextAfterHex}</pre>
                      {m.nearbyStrings.length > 0 ? (
                        <p>
                          {t("fmParser.nearbyStrings")}:{" "}
                          {m.nearbyStrings.join(" · ")}
                        </p>
                      ) : null}
                      <ul className="save-analyse__int-list">
                        {m.nearbyIntegers.slice(0, 20).map((i) => (
                          <li
                            key={`${i.offset}-${i.width}-${i.endian}-${i.signed}-${i.value}`}
                          >
                            @{i.offset} {i.signed ? "i" : "u"}
                            {i.width}/{i.endian} = {i.value}
                          </li>
                        ))}
                      </ul>
                    </details>
                  ))}
                </div>
              </div>
            ) : null}
          </section>

          {/* G. Performance */}
          <section className="save-analyse__panel save-analyse__panel--wide">
            <h3>{t("fmParser.perfTitle")}</h3>
            {engine ? (
              <dl className="save-analyse__meta-grid">
                <div>
                  <dt>{t("fmParser.parseDuration")}</dt>
                  <dd>{(engine.performance.elapsedMs / 1000).toFixed(2)} s</dd>
                </div>
                <div>
                  <dt>{t("fmParser.mbPerS")}</dt>
                  <dd>{engine.performance.mbPerS.toFixed(2)}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.bytesProcessed")}</dt>
                  <dd>{formatBytes(engine.performance.bytesProcessed)}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.decompressedBytes")}</dt>
                  <dd>{formatBytes(engine.performance.decompressedBytes)}</dd>
                </div>
                <div>
                  <dt>{t("fmParser.cacheHit")}</dt>
                  <dd>
                    {engine.fromCache || engine.performance.cacheHit
                      ? t("fmParser.yes")
                      : t("fmParser.no")}
                  </dd>
                </div>
              </dl>
            ) : null}
            <div className="save-analyse__actions" style={{ marginTop: 12 }}>
              <Button
                type="button"
                variant="secondary"
                disabled={busy || !activePath}
                onClick={() => void onExport()}
              >
                {t("fmParser.exportDiag")}
              </Button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
