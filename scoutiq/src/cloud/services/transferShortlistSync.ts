import type { TransferShortlistItem } from "../types";
import type { TransferShortlistSyncService } from "./types";
import { getCloudProvider } from "../syncEngine";

export function getTransferShortlistSyncService(): TransferShortlistSyncService {
  return getCloudProvider().transferShortlist;
}

export async function readLocalTransferShortlist(): Promise<
  TransferShortlistItem[]
> {
  return getCloudProvider().transferShortlist.listLocal();
}
