const STORAGE_KEY = "scoutiq.developerMode";

export function isDeveloperMode(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

export function setDeveloperMode(enabled: boolean): void {
  try {
    localStorage.setItem(STORAGE_KEY, enabled ? "1" : "0");
    window.dispatchEvent(
      new CustomEvent("scoutiq://developer-mode", { detail: enabled }),
    );
  } catch {
    // ignore
  }
}

export function subscribeDeveloperMode(
  cb: (enabled: boolean) => void,
): () => void {
  const handler = (e: Event) => {
    const ce = e as CustomEvent<boolean>;
    cb(Boolean(ce.detail));
  };
  window.addEventListener("scoutiq://developer-mode", handler);
  return () => window.removeEventListener("scoutiq://developer-mode", handler);
}
