import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DiagnosticExport,
  EngineProgress,
  FmEngineResult,
  FmSaveDiff,
  KnownValueProbeResult,
  ProbeType,
} from "./fmContainer";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function parseFmContainer(path: string): Promise<FmEngineResult> {
  if (!isTauriRuntime()) {
    throw new Error("Nur in der Desktop-App verfügbar");
  }
  return invoke<FmEngineResult>("parse_fm_container", { path });
}

export async function startFmEngineAnalysis(path: string): Promise<void> {
  if (!isTauriRuntime()) {
    throw new Error("Nur in der Desktop-App verfügbar");
  }
  return invoke("start_fm_engine_analysis", { path });
}

export async function cancelFmAnalysis(): Promise<void> {
  if (!isTauriRuntime()) return;
  await invoke("cancel_fm_analysis");
}

export async function probeFmContainer(
  path: string,
  query: string,
  queryType: ProbeType | string,
): Promise<KnownValueProbeResult> {
  if (!isTauriRuntime()) {
    throw new Error("Nur in der Desktop-App verfügbar");
  }
  return invoke<KnownValueProbeResult>("probe_fm_container", {
    path,
    query,
    queryType,
  });
}

export async function compareFmSaves(
  pathA: string,
  pathB: string,
): Promise<FmSaveDiff> {
  if (!isTauriRuntime()) {
    throw new Error("Nur in der Desktop-App verfügbar");
  }
  return invoke<FmSaveDiff>("compare_fm_saves", { pathA, pathB });
}

export async function exportFmDiagnostic(
  path: string,
  query?: string,
  queryType?: string,
): Promise<DiagnosticExport> {
  if (!isTauriRuntime()) {
    throw new Error("Nur in der Desktop-App verfügbar");
  }
  return invoke<DiagnosticExport>("export_fm_diagnostic", {
    path,
    query: query ?? null,
    queryType: queryType ?? null,
  });
}

export async function listenEngineProgress(
  onProgress: (p: EngineProgress) => void,
): Promise<UnlistenFn> {
  return listen<EngineProgress>("scoutiq://fm-progress", (event) => {
    onProgress(event.payload);
  });
}

export async function listenEngineDone(
  onDone: (payload: {
    ok: boolean;
    result?: FmEngineResult;
    identity?: import("./currentSavePersistence").FmSaveAnalysisResultDto;
    error?: string;
  }) => void,
): Promise<UnlistenFn> {
  return listen<{
    ok: boolean;
    result?: FmEngineResult;
    identity?: import("./currentSavePersistence").FmSaveAnalysisResultDto;
    error?: string;
  }>("scoutiq://fm-engine-done", (event) => {
    onDone(event.payload);
  });
}
