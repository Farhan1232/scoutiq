import type { AnalysisHistoryItem } from "../types";
import type { AnalysisHistorySyncService } from "./types";
import { getCloudProvider } from "../syncEngine";

export function getAnalysisHistorySyncService(): AnalysisHistorySyncService {
  return getCloudProvider().analysisHistory;
}

export async function readLocalAnalysisHistory(): Promise<AnalysisHistoryItem[]> {
  return getCloudProvider().analysisHistory.listLocal();
}
