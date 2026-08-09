import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { FmSaveAnalysisResultDto } from "./currentSavePersistence";

function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export interface IdentityProgress {
  stage: string;
  progress: number;
  message: string;
  framesScanned: number;
  framesTotal: number;
}

export async function analyzeFmIdentity(
  path: string,
): Promise<FmSaveAnalysisResultDto> {
  if (!isTauriRuntime()) {
    throw new Error("Nur in der Desktop-App verfügbar");
  }
  return invoke<FmSaveAnalysisResultDto>("analyze_fm_identity", { path });
}

export async function listenIdentityProgress(
  onProgress: (progress: IdentityProgress) => void,
): Promise<UnlistenFn> {
  return listen<IdentityProgress>("scoutiq://fm-identity-progress", (event) => {
    onProgress(event.payload);
  });
}
