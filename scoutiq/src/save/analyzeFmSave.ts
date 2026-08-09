import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AnalysisDiagnostics,
  AnalysisProgress,
  FmQuickValidation,
} from "./fmAnalysisTypes";
import type { ForensicReport, ProbeQueryType, ProbeResult } from "./fmForensic";
import type { FmSaveInspection } from "./fmInspection";
import type { FmSaveMetadata } from "./fmMetadata";
import type { SaveError } from "./types";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export interface FmSaveAnalysis {
  inspection: FmSaveInspection;
  metadata: FmSaveMetadata;
  diagnostics: AnalysisDiagnostics;
  forensic: ForensicReport;
}

export interface AnalysisDonePayload {
  ok: boolean;
  analysis: FmSaveAnalysis | null;
  error: string | null;
}

export type AnalyzeFmSaveResult =
  | { ok: true; analysis: FmSaveAnalysis }
  | { ok: false; error: SaveError };

function toSaveError(detail: string): SaveError {
  return {
    code: "read_failed",
    messageKey: "save.errors.inspectFailed",
    detail,
  };
}

/** Stage 1 — immediate validation, no long scan. */
export async function validateFmSave(
  path: string,
): Promise<{ ok: true; validation: FmQuickValidation } | { ok: false; error: SaveError }> {
  if (!isTauriRuntime()) {
    return {
      ok: false,
      error: { code: "unknown", messageKey: "save.errors.desktopOnly" },
    };
  }
  try {
    const validation = await invoke<FmQuickValidation>("validate_fm_save", { path });
    return { ok: true, validation };
  } catch (error) {
    return {
      ok: false,
      error: toSaveError(error instanceof Error ? error.message : String(error)),
    };
  }
}

/**
 * Starts staged Rust analysis (no frontend timeout).
 * Progress/done arrive via events; this only kicks off the job.
 */
export async function startFmAnalysis(
  path: string,
): Promise<{ ok: true } | { ok: false; error: SaveError }> {
  if (!isTauriRuntime()) {
    return {
      ok: false,
      error: { code: "unknown", messageKey: "save.errors.desktopOnly" },
    };
  }
  try {
    await invoke("start_fm_analysis", { path });
    return { ok: true };
  } catch (error) {
    return {
      ok: false,
      error: toSaveError(error instanceof Error ? error.message : String(error)),
    };
  }
}

export async function cancelFmAnalysis(): Promise<void> {
  if (!isTauriRuntime()) return;
  try {
    await invoke("cancel_fm_analysis");
  } catch (error) {
    console.warn("[save] cancel_fm_analysis failed:", error);
  }
}

/** Developer known-value probe (ASCII / UTF-8 / UTF-16 LE+BE, raw + zstd). */
export async function probeFmValue(
  path: string,
  query: string,
  queryType: ProbeQueryType | string,
): Promise<{ ok: true; result: ProbeResult } | { ok: false; error: SaveError }> {
  if (!isTauriRuntime()) {
    return {
      ok: false,
      error: { code: "unknown", messageKey: "save.errors.desktopOnly" },
    };
  }
  try {
    const result = await invoke<ProbeResult>("probe_fm_value", {
      path,
      query,
      queryType,
    });
    return { ok: true, result };
  } catch (error) {
    return {
      ok: false,
      error: toSaveError(error instanceof Error ? error.message : String(error)),
    };
  }
}

export async function listenFmAnalysisProgress(
  onProgress: (progress: AnalysisProgress) => void,
): Promise<UnlistenFn> {
  return listen<AnalysisProgress>("scoutiq://fm-progress", (event) => {
    onProgress(event.payload);
  });
}

export async function listenFmAnalysisDone(
  onDone: (payload: AnalysisDonePayload) => void,
): Promise<UnlistenFn> {
  return listen<AnalysisDonePayload>("scoutiq://fm-analysis-done", (event) => {
    onDone(event.payload);
  });
}
