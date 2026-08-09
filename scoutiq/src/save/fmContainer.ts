export type ContainerType = "fmf" | "unknown";

export type CompressionType =
  | "zstandard"
  | "gzip"
  | "zlib"
  | "deflate"
  | "zip"
  | "highEntropy"
  | "unknown";

export type ValidationStatus =
  | "candidate"
  | "valid"
  | "invalid"
  | "processed"
  | "error";

export type StringEncoding = "ascii" | "utf8" | "utf16Le" | "utf16Be";

export type StructureType =
  | "stringTable"
  | "recordTable"
  | "indexTable"
  | "referenceTable"
  | "internalFile"
  | "entityTable"
  | "unknownRepeatedStructure";

export type Confidence = "high" | "medium" | "low";

export interface FmFileInfo {
  fileName: string;
  filePath: string;
  fileSize: number;
  headerHex: string;
  detectedContainer: ContainerType | null;
  modifiedUnix: number | null;
}

export interface CompressedBlockInfo {
  index: number;
  compressionType: CompressionType;
  fileOffset: number;
  compressedSize: number | null;
  decompressedSize: number | null;
  validationStatus: ValidationStatus;
  error: string | null;
  contentDigest: string | null;
  signatures: string[];
  stringCount: number;
}

export interface FmStringEntry {
  value: string;
  encoding: StringEncoding;
  blockIndex: number;
  decompressedOffset: number;
  sourceFileOffset: number;
}

export interface StructureCandidate {
  blockIndex: number;
  offset: number;
  structureType: StructureType;
  confidence: Confidence;
  evidence: string[];
}

export interface BlockStats {
  candidates: number;
  validated: number;
  decompressed: number;
  errors: number;
}

export interface StructureStats {
  stringTables: number;
  recordTables: number;
  indexTables: number;
  internalFiles: number;
  unknown: number;
}

export interface PerformanceDiagnostics {
  elapsedMs: number;
  bytesProcessed: number;
  decompressedBytes: number;
  blocksProcessed: number;
  mbPerS: number;
  cacheHit: boolean;
  errors: number;
}

/** Primary engine result — no invented game metadata. */
export interface FmEngineResult {
  file: FmFileInfo;
  blocks: CompressedBlockInfo[];
  blockStats: BlockStats;
  structures: StructureCandidate[];
  structureStats: StructureStats;
  stringIndex: FmStringEntry[];
  stringEntries: number;
  performance: PerformanceDiagnostics;
  parserStatus: string;
  analysisStatus: string;
  notes: string[];
  fromCache: boolean;
}

/** @deprecated alias */
export type ContainerParseResult = FmEngineResult;

export interface NearbyInteger {
  offset: number;
  width: number;
  endian: string;
  signed: boolean;
  value: string;
}

export interface KnownValueMatch {
  searchedValue: string;
  blockIndex: number;
  sourceFileOffset: number;
  decompressedOffset: number;
  encoding: StringEncoding | string;
  nearbyStrings: string[];
  contextBeforeHex: string;
  contextAfterHex: string;
  nearbyIntegers: NearbyInteger[];
}

export interface KnownValueProbeResult {
  query: string;
  queryType: string;
  matches: KnownValueMatch[];
  blocksSearched: number;
  notes: string[];
}

export interface ChangedBlock {
  indexA: number | null;
  indexB: number | null;
  fileOffsetA: number | null;
  fileOffsetB: number | null;
  status: string;
  digestA: string | null;
  digestB: string | null;
  decompressedSizeA: number | null;
  decompressedSizeB: number | null;
  note: string;
}

export interface ChangedRegion {
  blockIndex: number | null;
  offset: number;
  length: number;
  note: string;
}

export interface FmSaveDiff {
  fileASize: number;
  fileBSize: number;
  blocksA: number;
  blocksB: number;
  pathA: string;
  pathB: string;
  changedBlocks: ChangedBlock[];
  changedRegions: ChangedRegion[];
  unchangedCount: number;
  onlyInA: number;
  onlyInB: number;
  notes: string[];
}

/** @deprecated alias */
export type SaveDiffResult = FmSaveDiff;

export type DiagnosticExport = Record<string, unknown>;

export interface EngineProgress {
  stage: string;
  progress: number;
  currentBlock: number;
  totalBlocks: number;
  bytesProcessed: number;
  totalBytes: number;
  message: string;
}

export type ProbeType = "manager" | "club" | "competition" | "player" | "free";
