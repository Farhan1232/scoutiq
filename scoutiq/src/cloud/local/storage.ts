import { CLOUD_STORAGE_PREFIX } from "../config";

export function localKey(parts: string[]): string {
  return `${CLOUD_STORAGE_PREFIX}${parts.join(".")}`;
}

export function readLocalJson<T>(key: string, fallback: T): T {
  if (typeof window === "undefined") return fallback;
  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) return fallback;
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

export function writeLocalJson(key: string, value: unknown): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(key, JSON.stringify(value));
}

export function removeLocalKey(key: string): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(key);
}
