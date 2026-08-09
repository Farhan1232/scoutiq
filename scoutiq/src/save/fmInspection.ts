/** Result of the Rust `inspect_fm_save` research/debug command. */
export interface FmSaveInspection {
  fileName: string;
  filePath: string;
  fileSize: number;
  firstBytesHex: string;
  detectedCompression: string;
  zstdFrameCount: number;
  readableStringsSample: string[];
  errors: string[];
  detectedSignatures?: string[];
}
