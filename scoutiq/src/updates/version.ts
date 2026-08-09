import { getVersion } from "@tauri-apps/api/app";
import { APP_VERSION_FALLBACK } from "./config";
import { isTauriRuntime } from "./runtime";

export async function readAppVersion(): Promise<string> {
  if (!isTauriRuntime()) {
    return APP_VERSION_FALLBACK;
  }

  try {
    return await getVersion();
  } catch {
    return APP_VERSION_FALLBACK;
  }
}
