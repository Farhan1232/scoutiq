import type { SyncableEntity, ConflictStrategy, SyncConflict } from "./types";

/**
 * Compare updated_at timestamps for conflict detection.
 * Does not mutate data — callers decide how to resolve.
 */
export function compareUpdatedAt(a: string, b: string): number {
  return new Date(a).getTime() - new Date(b).getTime();
}

export function isRemoteNewer<T extends SyncableEntity>(
  local: T,
  remote: T,
): boolean {
  return compareUpdatedAt(remote.updatedAt, local.updatedAt) > 0;
}

export function isLocalNewer<T extends SyncableEntity>(
  local: T,
  remote: T,
): boolean {
  return compareUpdatedAt(local.updatedAt, remote.updatedAt) > 0;
}

/**
 * Detect a write conflict when both sides changed after the last known sync.
 * Same timestamp is treated as already aligned.
 */
export function detectConflict<T extends SyncableEntity>(
  domain: SyncConflict<T>["domain"],
  local: T,
  remote: T,
): SyncConflict<T> | null {
  if (local.id !== remote.id) return null;
  if (local.updatedAt === remote.updatedAt) return null;

  const suggested: ConflictStrategy = isRemoteNewer(local, remote)
    ? "remote_wins"
    : "local_wins";

  return {
    domain,
    entityId: local.id,
    local,
    remote,
    suggested,
  };
}

export function resolveByUpdatedAt<T extends SyncableEntity>(
  local: T,
  remote: T,
): T {
  return isRemoteNewer(local, remote) ? remote : local;
}

export function nowIso(): string {
  return new Date().toISOString();
}
