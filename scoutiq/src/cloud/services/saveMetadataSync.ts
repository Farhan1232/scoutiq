import type { SaveMetadataSyncData } from "../types";
import type { SaveMetadataSyncService } from "./types";
import { getCloudProvider } from "../syncEngine";

export function getSaveMetadataSyncService(): SaveMetadataSyncService {
  return getCloudProvider().saveMetadata;
}

export async function readLocalSaveMetadata(): Promise<SaveMetadataSyncData[]> {
  return getCloudProvider().saveMetadata.listLocal();
}
