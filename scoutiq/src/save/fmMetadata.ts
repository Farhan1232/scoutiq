export type MetadataConfidence =
  | "confirmed"
  | "likely"
  | "uncertain"
  | "unknown";

export interface MetadataField {
  value: string | null;
  confidence: MetadataConfidence;
  evidence: string[];
}

export interface FmSaveMetadataFields {
  managerName: MetadataField;
  clubName: MetadataField;
  clubId: MetadataField;
  inGameDate: MetadataField;
  season: MetadataField;
  competitionName: MetadataField;
  saveName: MetadataField;
}

export interface MetadataConfidenceMap {
  managerName: MetadataConfidence;
  clubName: MetadataConfidence;
  clubId: MetadataConfidence;
  inGameDate: MetadataConfidence;
  season: MetadataConfidence;
  competitionName: MetadataConfidence;
  saveName: MetadataConfidence;
}

/** Result of Rust `extract_fm_metadata`. */
export interface FmSaveMetadata {
  managerName: string | null;
  clubName: string | null;
  clubId: string | null;
  inGameDate: string | null;
  season: string | null;
  competitionName: string | null;
  saveName: string | null;
  confidence: MetadataConfidenceMap;
  fields: FmSaveMetadataFields;
  notes: string[];
  errors: string[];
}
