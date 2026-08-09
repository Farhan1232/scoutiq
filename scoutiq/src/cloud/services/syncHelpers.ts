import { detectConflict, isRemoteNewer, resolveByUpdatedAt } from "../conflict";
import type { SyncDomain } from "../config";
import type { DomainSyncResult, SyncConflict, SyncableEntity } from "../types";

interface SyncCollectionParams<T extends SyncableEntity, R> {
  domain: SyncDomain;
  localItems: T[];
  remoteItems: T[];
  toRow: (item: T) => R;
  upsertRemote: (rows: R[]) => Promise<void>;
  setLocal: (items: T[]) => void;
}

/**
 * Merge local cache + remote rows using updated_at (newer wins).
 * Soft-deleted rows are kept for propagation then filtered from active cache.
 */
export async function syncCollectionByUpdatedAt<
  T extends SyncableEntity,
  R,
>({
  domain,
  localItems,
  remoteItems,
  toRow,
  upsertRemote,
  setLocal,
}: SyncCollectionParams<T, R>): Promise<{
  result: DomainSyncResult;
  conflicts: SyncConflict<T>[];
}> {
  const localMap = new Map(localItems.map((item) => [item.id, item]));
  const remoteMap = new Map(remoteItems.map((item) => [item.id, item]));
  const ids = new Set([...localMap.keys(), ...remoteMap.keys()]);

  const merged: T[] = [];
  const toPush: T[] = [];
  const conflicts: SyncConflict<T>[] = [];
  let pushed = 0;
  let pulled = 0;

  for (const id of ids) {
    const local = localMap.get(id);
    const remote = remoteMap.get(id);

    if (local && remote) {
      const conflict = detectConflict(domain, local, remote);
      if (conflict) conflicts.push(conflict);

      const winner = resolveByUpdatedAt(local, remote);
      merged.push(winner);

      if (isRemoteNewer(local, remote)) {
        pulled += 1;
      } else if (local.updatedAt !== remote.updatedAt) {
        toPush.push(local);
        pushed += 1;
      }
      continue;
    }

    if (local && !remote) {
      merged.push(local);
      toPush.push(local);
      pushed += 1;
      continue;
    }

    if (!local && remote) {
      merged.push(remote);
      pulled += 1;
    }
  }

  if (toPush.length > 0) {
    await upsertRemote(toPush.map(toRow));
  }

  // Active cache excludes soft-deleted rows.
  const active = merged.filter((item) => !item.deletedAt);
  setLocal(active);

  return {
    conflicts,
    result: {
      domain,
      pushed,
      pulled,
      conflicts: conflicts.length,
      status: "synced",
      messageKey: "cloud.status.synced",
    },
  };
}
