import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  cancelFmAnalysis,
  listenFmAnalysisDone,
  listenFmAnalysisProgress,
  startFmAnalysis,
  type FmSaveAnalysis,
} from "./analyzeFmSave";
import type { AnalysisProgress } from "./fmAnalysisTypes";
import type { FmSaveInspection } from "./fmInspection";
import type { FmSaveMetadata } from "./fmMetadata";
import { loadFmSaveMetadata, openFmSaveWithPicker } from "./fmSaveService";
import {
  createEmptyProject,
  saveScoutIqProject,
  saveScoutIqProjectAs,
} from "./projectService";
import {
  loadRecentFmSaves,
  pushRecentFmSave,
  recentMenuLabel,
} from "./recentSaves";
import type {
  FmSaveRef,
  RecentFmSave,
  SaveError,
  SaveLoadState,
  ScoutIqProject,
} from "./types";
import type { CurrentSaveContext } from "./currentSaveTypes";
import { EMPTY_CURRENT_SAVE, displayClubName } from "./currentSaveTypes";
import {
  clearPersistedIdentity,
  identityToContext,
  loadingContext,
  loadPersistedIdentity,
  persistIdentity,
  type FmSaveAnalysisResultDto,
} from "./currentSavePersistence";
import { analyzeFmIdentity, listenIdentityProgress, type IdentityProgress } from "./fmIdentityApi";
import { listenEngineDone, startFmEngineAnalysis } from "./fmContainerApi";

interface SaveContextValue {
  fmSave: FmSaveRef | null;
  currentSave: CurrentSaveContext;
  identityProgress: IdentityProgress | null;
  inspection: FmSaveInspection | null;
  metadata: FmSaveMetadata | null;
  diagnostics: FmSaveAnalysis["diagnostics"] | null;
  forensic: FmSaveAnalysis["forensic"] | null;
  analysisProgress: AnalysisProgress | null;
  isAnalyzing: boolean;
  loadState: SaveLoadState;
  lastError: SaveError | null;
  project: ScoutIqProject;
  projectPath: string | null;
  recent: RecentFmSave[];
  openFmSave: () => Promise<void>;
  openRecentFmSave: (path: string) => Promise<void>;
  reinspectCurrentSave: () => Promise<void>;
  cancelAnalysis: () => Promise<void>;
  saveProject: () => Promise<void>;
  saveProjectAs: () => Promise<void>;
  clearError: () => void;
}

const SaveContext = createContext<SaveContextValue | null>(null);

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function syncRecentMenu(recent: RecentFmSave[]) {
  if (!isTauriRuntime()) return;
  try {
    await invoke("update_recent_menu", {
      items: recent.map((item) => ({
        path: item.path,
        label: recentMenuLabel(item.path, item.fileName),
      })),
    });
  } catch (error) {
    console.warn("[save] update_recent_menu failed:", error);
  }
}

async function updateWindowTitle(ctx: CurrentSaveContext) {
  if (!isTauriRuntime()) return;
  const club = displayClubName(ctx);
  const title =
    club && (ctx.status === "ready" || ctx.status === "partial")
      ? `ScoutIQ — ${club}`
      : "ScoutIQ";
  try {
    await getCurrentWindow().setTitle(title);
  } catch {
    // ignore
  }
}

function applyIdentityResult(
  identity: FmSaveAnalysisResultDto,
): CurrentSaveContext {
  const hasClub =
    !!identity.managed?.clubName &&
    ["confirmed", "high", "medium"].includes(
      (
        identity.confidence?.club ??
        identity.managed.clubConfidence ??
        ""
      ).toLowerCase(),
    );
  const hasComp =
    !!identity.managed?.competitionName &&
    ["confirmed", "high", "medium"].includes(
      (
        identity.confidence?.competition ??
        identity.managed.competitionConfidence ??
        ""
      ).toLowerCase(),
    );
  const status: CurrentSaveContext["status"] =
    hasClub && hasComp ? "ready" : hasClub || hasComp ? "partial" : "ready";
  return identityToContext(identity, status);
}

export function SaveProvider({ children }: { children: ReactNode }) {
  const [fmSave, setFmSave] = useState<FmSaveRef | null>(null);
  const [currentSave, setCurrentSave] =
    useState<CurrentSaveContext>(EMPTY_CURRENT_SAVE);
  const [identityProgress, setIdentityProgress] =
    useState<IdentityProgress | null>(null);
  const [inspection, setInspection] = useState<FmSaveInspection | null>(null);
  const [metadata, setMetadata] = useState<FmSaveMetadata | null>(null);
  const [diagnostics, setDiagnostics] = useState<
    FmSaveAnalysis["diagnostics"] | null
  >(null);
  const [forensic, setForensic] = useState<FmSaveAnalysis["forensic"] | null>(
    null,
  );
  const [analysisProgress, setAnalysisProgress] =
    useState<AnalysisProgress | null>(null);
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [loadState, setLoadState] = useState<SaveLoadState>("none");
  const [lastError, setLastError] = useState<SaveError | null>(null);
  const [projectPath, setProjectPath] = useState<string | null>(null);
  const [project, setProject] = useState<ScoutIqProject>(() =>
    createEmptyProject("Unbenanntes Projekt"),
  );
  const [recent, setRecent] = useState<RecentFmSave[]>(() => loadRecentFmSaves());
  const analyzingPathRef = useRef<string | null>(null);

  useEffect(() => {
    void syncRecentMenu(recent);
  }, [recent]);

  useEffect(() => {
    void updateWindowTitle(currentSave);
  }, [currentSave]);

  useEffect(() => {
    const cached = loadPersistedIdentity();
    if (!cached) return;
    void (async () => {
      const meta = await loadFmSaveMetadata(cached.path);
      if (!meta.ok) {
        clearPersistedIdentity();
        return;
      }
      if (meta.save.fileSizeBytes !== cached.fileSize) {
        clearPersistedIdentity();
        return;
      }
      setFmSave(meta.save);
      setCurrentSave(cached.context);
      setLoadState("loaded");
      try {
        setIdentityProgress({
          stage: "opening",
          progress: 1,
          message: "Spielstand wird geöffnet …",
          framesScanned: 0,
          framesTotal: 0,
        });
        const identity = await analyzeFmIdentity(cached.path);
        const next = applyIdentityResult(identity);
        setCurrentSave(next);
        setIdentityProgress(null);
        persistIdentity({
          path: cached.path,
          fileName: meta.save.fileName,
          fileSize: meta.save.fileSizeBytes,
          fingerprint: next.fingerprint ?? cached.fingerprint,
          modifiedUnix: null,
          context: next,
          savedAt: new Date().toISOString(),
        });
      } catch {
        setIdentityProgress(null);
        // keep cached display
      }
    })();
  }, []);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let unlistenProgress: (() => void) | undefined;
    let unlistenDone: (() => void) | undefined;
    let unlistenEngine: (() => void) | undefined;
    let unlistenIdentity: (() => void) | undefined;

    void (async () => {
      unlistenProgress = await listenFmAnalysisProgress((progress) => {
        setAnalysisProgress(progress);
        if (progress.fileSize > 0) {
          setFmSave((prev) =>
            prev && prev.fileSizeBytes !== progress.fileSize
              ? { ...prev, fileSizeBytes: progress.fileSize }
              : prev,
          );
          setInspection((prev) =>
            prev && prev.fileSize !== progress.fileSize
              ? { ...prev, fileSize: progress.fileSize }
              : prev,
          );
        }
      });
      unlistenIdentity = await listenIdentityProgress((progress) => {
        setIdentityProgress(
          progress.stage === "done" ? null : progress,
        );
      });
      unlistenDone = await listenFmAnalysisDone((payload) => {
        setIsAnalyzing(false);
        analyzingPathRef.current = null;
        if (!payload.ok || !payload.analysis) {
          setLastError({
            code: "read_failed",
            messageKey: "save.errors.inspectFailed",
            detail: payload.error ?? "Unbekannter Analysefehler",
          });
          return;
        }

        const {
          inspection: nextInspection,
          metadata: nextMetadata,
          diagnostics: nextDiag,
          forensic: nextForensic,
        } = payload.analysis;
        setInspection(nextInspection);
        setMetadata(nextMetadata);
        setDiagnostics(nextDiag);
        setForensic(nextForensic ?? null);
        setFmSave((prev) =>
          prev
            ? {
                ...prev,
                fileSizeBytes: nextInspection.fileSize || prev.fileSizeBytes,
                fileName: nextInspection.fileName || prev.fileName,
              }
            : prev,
        );
        setAnalysisProgress({
          stage: "finished",
          progress: 100,
          message: "Analyse abgeschlossen",
          bytesScanned: nextDiag.bytesScanned,
          fileSize: nextDiag.fileSize,
          elapsedMs: nextDiag.totalScanMs,
        });

        const warnings = [...nextInspection.errors, ...nextMetadata.errors];
        if (warnings.length > 0) {
          setLastError({
            code: "unsupported",
            messageKey: "save.errors.inspectWarnings",
            detail: warnings.join(" · "),
          });
        } else {
          setLastError(null);
        }
      });

      unlistenEngine = await listenEngineDone((payload) => {
        setIdentityProgress(null);
        if (!payload.ok) {
          setCurrentSave((prev) =>
            prev.filePath
              ? {
                  ...prev,
                  status: "error",
                  errorMessage: payload.error ?? "Unbekannter Parserfehler.",
                }
              : prev,
          );
          return;
        }
        const identity = (payload as { identity?: FmSaveAnalysisResultDto })
          .identity;
        if (identity) {
          const next = applyIdentityResult(identity);
          setCurrentSave(next);
          persistIdentity({
            path: next.filePath ?? "",
            fileName: next.fileName ?? "",
            fileSize: next.fileSize ?? 0,
            fingerprint: next.fingerprint ?? "",
            modifiedUnix: null,
            context: next,
            savedAt: new Date().toISOString(),
          });
        }
      });
    })();

    return () => {
      unlistenProgress?.();
      unlistenDone?.();
      unlistenEngine?.();
      unlistenIdentity?.();
    };
  }, []);

  const runIdentityResolution = useCallback(async (save: FmSaveRef) => {
    setIdentityProgress({
      stage: "opening",
      progress: 1,
      message: "Spielstand wird geöffnet …",
      framesScanned: 0,
      framesTotal: 0,
    });
    try {
      await startFmEngineAnalysis(save.path);
    } catch {
      try {
        const identity = await analyzeFmIdentity(save.path);
        const next = applyIdentityResult(identity);
        setCurrentSave(next);
        setIdentityProgress(null);
        persistIdentity({
          path: save.path,
          fileName: save.fileName,
          fileSize: save.fileSizeBytes,
          fingerprint: next.fingerprint ?? "",
          modifiedUnix: null,
          context: next,
          savedAt: new Date().toISOString(),
        });
      } catch (e) {
        setIdentityProgress(null);
        setCurrentSave((prev) => ({
          ...prev,
          status: "error",
          errorMessage: e instanceof Error ? e.message : String(e),
        }));
      }
    }
  }, []);

  const runAnalysis = useCallback(
    async (save: FmSaveRef) => {
      setLastError(null);
      setMetadata(null);
      setDiagnostics(null);
      setForensic(null);
      setIsAnalyzing(true);
      analyzingPathRef.current = save.path;
      setAnalysisProgress({
        stage: "opening_file",
        progress: 1,
        message: "Spielstand wird analysiert",
        bytesScanned: 0,
        fileSize: save.fileSizeBytes,
        elapsedMs: 0,
      });
      setInspection({
        fileName: save.fileName,
        filePath: save.path,
        fileSize: save.fileSizeBytes,
        firstBytesHex: "",
        detectedCompression: "—",
        zstdFrameCount: 0,
        readableStringsSample: [],
        errors: [],
        detectedSignatures: [],
      });

      void runIdentityResolution(save);

      const started = await startFmAnalysis(save.path);
      if (!started.ok) {
        setIsAnalyzing(false);
        analyzingPathRef.current = null;
        setLastError(started.error);
      }
    },
    [runIdentityResolution],
  );

  const applyLoadedSave = useCallback(
    (save: FmSaveRef) => {
      clearPersistedIdentity();
      setCurrentSave(loadingContext(save));
      setFmSave(save);
      setInspection(null);
      setMetadata(null);
      setDiagnostics(null);
      setForensic(null);
      setLoadState("loaded");
      setLastError(null);
      setProject((prev) => ({
        ...prev,
        fmSave: save,
        updatedAt: new Date().toISOString(),
        name: prev.name === "Unbenanntes Projekt" ? save.fileName : prev.name,
      }));

      const nextRecent = pushRecentFmSave({
        path: save.path,
        fileName: save.fileName,
        openedAt: new Date().toISOString(),
      });
      setRecent(nextRecent);

      void runAnalysis(save);
    },
    [runAnalysis],
  );

  const openFmSave = useCallback(async () => {
    setLastError(null);
    const result = await openFmSaveWithPicker();
    if (!result.ok) {
      if (result.error.code === "cancelled") {
        return;
      }
      setLoadState("error");
      setLastError(result.error);
      return;
    }
    applyLoadedSave(result.save);
  }, [applyLoadedSave]);

  const openRecentFmSave = useCallback(
    async (path: string) => {
      setLastError(null);
      const result = await loadFmSaveMetadata(path);
      if (!result.ok) {
        setLoadState("error");
        setLastError(result.error);
        return;
      }
      applyLoadedSave(result.save);
    },
    [applyLoadedSave],
  );

  const reinspectCurrentSave = useCallback(async () => {
    if (!fmSave) return;
    setCurrentSave(loadingContext(fmSave));
    await runAnalysis(fmSave);
  }, [fmSave, runAnalysis]);

  const cancelAnalysis = useCallback(async () => {
    await cancelFmAnalysis();
    setIsAnalyzing(false);
    analyzingPathRef.current = null;
    setAnalysisProgress(null);
    setLastError({
      code: "cancelled",
      messageKey: "save.errors.cancelledAnalysis",
      detail: "Analyse wurde abgebrochen",
    });
  }, []);

  const saveProject = useCallback(async () => {
    const result = await saveScoutIqProject(project, projectPath);
    if (!result.ok) {
      if (result.error.code !== "cancelled") setLastError(result.error);
      return;
    }
    setProject(result.project);
    setProjectPath(result.path);
    setLastError(null);
  }, [project, projectPath]);

  const saveProjectAs = useCallback(async () => {
    const result = await saveScoutIqProjectAs(project);
    if (!result.ok) {
      if (result.error.code !== "cancelled") setLastError(result.error);
      return;
    }
    setProject(result.project);
    setProjectPath(result.path);
    setLastError(null);
  }, [project]);

  useEffect(() => {
    if (!isTauriRuntime()) return;

    let unlistenMenu: (() => void) | undefined;
    let unlistenRecent: (() => void) | undefined;

    void (async () => {
      unlistenMenu = await listen<string>("scoutiq://menu", (event) => {
        const action = event.payload;
        if (action === "open-fm-save") void openFmSave();
        if (action === "save-project") void saveProject();
        if (action === "save-project-as") void saveProjectAs();
        if (action === "view-reload") window.location.reload();
      });

      unlistenRecent = await listen<string>("scoutiq://open-recent", (event) => {
        void openRecentFmSave(event.payload);
      });
    })();

    return () => {
      unlistenMenu?.();
      unlistenRecent?.();
    };
  }, [openFmSave, openRecentFmSave, saveProject, saveProjectAs]);

  const value = useMemo(
    () => ({
      fmSave,
      currentSave,
      identityProgress,
      inspection,
      metadata,
      diagnostics,
      forensic,
      analysisProgress,
      isAnalyzing,
      loadState,
      lastError,
      project,
      projectPath,
      recent,
      openFmSave,
      openRecentFmSave,
      reinspectCurrentSave,
      cancelAnalysis,
      saveProject,
      saveProjectAs,
      clearError: () => setLastError(null),
    }),
    [
      fmSave,
      currentSave,
      identityProgress,
      inspection,
      metadata,
      diagnostics,
      forensic,
      analysisProgress,
      isAnalyzing,
      loadState,
      lastError,
      project,
      projectPath,
      recent,
      openFmSave,
      openRecentFmSave,
      reinspectCurrentSave,
      cancelAnalysis,
      saveProject,
      saveProjectAs,
    ],
  );

  return <SaveContext.Provider value={value}>{children}</SaveContext.Provider>;
}

export function useSave() {
  const context = useContext(SaveContext);
  if (!context) {
    throw new Error("useSave must be used within SaveProvider");
  }
  return context;
}

export function useCurrentSave() {
  return useSave().currentSave;
}
