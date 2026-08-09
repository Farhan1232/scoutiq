import { useCallback, useEffect, useState } from "react";
import { APP_VERSION_FALLBACK } from "./config";
import type { UpdateStatusKind } from "./types";
import { checkForAppUpdates, getCurrentAppVersion } from "./updateService";

interface UseUpdatesState {
  version: string;
  status: UpdateStatusKind;
  messageKey: string;
  messageParams?: Record<string, string>;
  availableVersion?: string;
  isChecking: boolean;
  checkForUpdates: () => Promise<void>;
}

export function useUpdates(): UseUpdatesState {
  const [version, setVersion] = useState(APP_VERSION_FALLBACK);
  const [status, setStatus] = useState<UpdateStatusKind>("idle");
  const [messageKey, setMessageKey] = useState("settings.updates.status.idle");
  const [messageParams, setMessageParams] = useState<
    Record<string, string> | undefined
  >();
  const [availableVersion, setAvailableVersion] = useState<string>();
  const [isChecking, setIsChecking] = useState(false);

  useEffect(() => {
    let active = true;

    void getCurrentAppVersion().then((next) => {
      if (active) {
        setVersion(next);
      }
    });

    return () => {
      active = false;
    };
  }, []);

  const checkForUpdates = useCallback(async () => {
    setIsChecking(true);
    setStatus("checking");
    setMessageKey("settings.updates.status.checking");
    setMessageParams(undefined);
    setAvailableVersion(undefined);

    try {
      const result = await checkForAppUpdates();
      setVersion(result.currentVersion);
      setStatus(result.status);
      setMessageKey(result.messageKey);
      setMessageParams(result.messageParams);
      setAvailableVersion(result.available?.version);
    } catch {
      setStatus("error");
      setMessageKey("settings.updates.status.error");
      setMessageParams({ detail: "Unexpected error" });
    } finally {
      setIsChecking(false);
    }
  }, []);

  return {
    version,
    status,
    messageKey,
    messageParams,
    availableVersion,
    isChecking,
    checkForUpdates,
  };
}
