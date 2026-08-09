export type AnalysisStage =
  | "opening_file"
  | "reading_header"
  | "scanning_structure"
  | "detecting_compression"
  | "extracting_metadata"
  | "finished";

export interface AnalysisProgress {
  stage: AnalysisStage | string;
  progress: number;
  message: string;
  bytesScanned: number;
  fileSize: number;
  elapsedMs: number;
}

export interface AnalysisDiagnostics {
  fileSize: number;
  bytesScanned: number;
  totalScanMs: number;
  readSpeedMbS: number;
  stage: string;
  chunkSize: number;
  stringSamples: number;
  zstdFrameCount: number;
}

export interface FmQuickValidation {
  fileName: string;
  filePath: string;
  fileSize: number;
  firstBytesHex: string;
  supportedExtension: boolean;
}
