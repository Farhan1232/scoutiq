export interface ForensicStringHit {
  offset: number;
  value: string;
  length: number;
  encoding: string;
  source: string;
  contextBeforeHex: string;
  contextAfterHex: string;
}

export interface IntegerCandidate {
  offset: number;
  width: number;
  endian: string;
  signed: boolean;
  value: string;
}

export interface ProbeMatch {
  offset: number;
  encoding: string;
  source: string;
  matchedValue: string;
  contextHex: string;
  nearbyStrings: string[];
  nearbyIntegers: IntegerCandidate[];
}

export interface RepeatedPatternGroup {
  value: string;
  occurrences: number;
  offsets: number[];
  similarContextCount: number;
  note: string;
}

export interface EntropyBlock {
  offset: number;
  size: number;
  entropy: number;
  kind: string;
}

export interface CompressedBlockInfo {
  offset: number;
  magic: string;
  decompressedBytes: number;
  success: boolean;
  note: string;
}

export interface EncodingStats {
  ascii: number;
  utf8: number;
  utf16Le: number;
  utf16Be: number;
  fromRaw: number;
  fromZstd: number;
}

export interface ForensicLog {
  bytesScanned: number;
  fileSize: number;
  stringsByEncoding: EncodingStats;
  compressedBlocksDetected: number;
  decompressedBlocksProcessed: number;
  decompressedBytesTotal: number;
  catalogEntries: number;
  repeatedGroups: number;
  entropyBlocks: number;
  promisingOffsets: number[];
  summaryLines: string[];
}

export interface ForensicReport {
  strings: ForensicStringHit[];
  repeatedPatterns: RepeatedPatternGroup[];
  entropyMap: EntropyBlock[];
  compressedBlocks: CompressedBlockInfo[];
  log: ForensicLog;
}

export interface ProbeResult {
  query: string;
  queryType: string;
  matches: ProbeMatch[];
  encodingsHit: string[];
  notes: string[];
}

export type ProbeQueryType = "manager" | "club" | "competition" | "free";
