import type { RecentFmSave } from "./types";

const STORAGE_KEY = "scoutiq.recent.fmSaves";
const MAX_RECENT = 8;

export function loadRecentFmSaves(): RecentFmSave[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as RecentFmSave[];
    return Array.isArray(parsed) ? parsed.slice(0, MAX_RECENT) : [];
  } catch {
    return [];
  }
}

export function pushRecentFmSave(entry: RecentFmSave): RecentFmSave[] {
  const next = [
    entry,
    ...loadRecentFmSaves().filter((item) => item.path !== entry.path),
  ].slice(0, MAX_RECENT);
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  return next;
}

export function recentMenuLabel(path: string, fileName: string): string {
  const truncated =
    path.length > 64 ? `…${path.slice(Math.max(0, path.length - 60))}` : path;
  return `${fileName} — ${truncated}`;
}
