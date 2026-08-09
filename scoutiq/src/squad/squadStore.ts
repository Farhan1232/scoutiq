import type { Squad } from "./types";

const STORAGE_PREFIX = "scoutiq.squad.";

function storageKey(userId: string): string {
  return `${STORAGE_PREFIX}${userId}`;
}

export function loadSquad(userId: string): Squad | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(storageKey(userId));
    if (!raw) return null;
    return JSON.parse(raw) as Squad;
  } catch {
    return null;
  }
}

export function saveSquad(userId: string, squad: Squad): void {
  window.localStorage.setItem(storageKey(userId), JSON.stringify(squad));
}

export function clearSquad(userId: string): void {
  window.localStorage.removeItem(storageKey(userId));
}
