import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { useAuth } from "../auth";
import { localCloudStore } from "./local/localCloudStore";
import {
  getInitialSyncStatus,
  getLastSyncedAt,
  resolveCloudModeBadge,
  runCloudSync,
} from "./syncEngine";
import type { CloudModeBadge, SyncRunResult, SyncStatus } from "./types";

interface CloudSyncContextValue {
  status: SyncStatus;
  lastSyncedAt: string | null;
  lastResult: SyncRunResult | null;
  isSyncing: boolean;
  modeBadge: CloudModeBadge;
  syncNow: () => Promise<void>;
  refreshConnectivity: () => void;
}

const CloudSyncContext = createContext<CloudSyncContextValue | null>(null);

interface CloudSyncProviderProps {
  children: ReactNode;
}

export function CloudSyncProvider({ children }: CloudSyncProviderProps) {
  const { isAuthenticated, user } = useAuth();
  const [status, setStatus] = useState<SyncStatus>(() => getInitialSyncStatus());
  const [lastSyncedAt, setLastSyncedAt] = useState<string | null>(null);
  const [lastResult, setLastResult] = useState<SyncRunResult | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);
  const [isOnline, setIsOnline] = useState(
    () => (typeof navigator === "undefined" ? true : navigator.onLine),
  );

  useEffect(() => {
    localCloudStore.clearLegacyMockCache();
  }, []);

  useEffect(() => {
    let active = true;
    void (async () => {
      if (!user?.id) {
        if (active) {
          setLastSyncedAt(null);
          setStatus(isOnline ? "unsigned" : "offline");
        }
        return;
      }
      const stamped = await getLastSyncedAt();
      if (active) {
        setLastSyncedAt(stamped);
        setStatus(isOnline ? "synced" : "offline");
      }
    })();
    return () => {
      active = false;
    };
  }, [user?.id, isOnline]);

  const refreshConnectivity = useCallback(() => {
    if (isSyncing) return;
    const online = navigator.onLine;
    setIsOnline(online);
    if (!online) {
      setStatus("offline");
      return;
    }
    setStatus(isAuthenticated ? "synced" : "unsigned");
  }, [isAuthenticated, isSyncing]);

  useEffect(() => {
    const onOnline = () => {
      setIsOnline(true);
      if (!isSyncing) {
        setStatus(isAuthenticated ? "synced" : "unsigned");
      }
    };
    const onOffline = () => {
      setIsOnline(false);
      setStatus("offline");
    };

    window.addEventListener("online", onOnline);
    window.addEventListener("offline", onOffline);
    return () => {
      window.removeEventListener("online", onOnline);
      window.removeEventListener("offline", onOffline);
    };
  }, [isAuthenticated, isSyncing]);

  const syncNow = useCallback(async () => {
    if (!isAuthenticated) {
      setStatus("unsigned");
      setLastResult({
        status: "unsigned",
        startedAt: new Date().toISOString(),
        finishedAt: new Date().toISOString(),
        domains: [],
        messageKey: "cloud.status.unsigned",
      });
      return;
    }

    setIsSyncing(true);
    setStatus("syncing");

    try {
      const result = await runCloudSync();
      setLastResult(result);
      setStatus(result.status);
      if (result.status === "synced") {
        setLastSyncedAt(result.finishedAt);
      } else {
        const stamped = await getLastSyncedAt();
        setLastSyncedAt(stamped);
      }
    } catch {
      setStatus("error");
      setLastResult(null);
    } finally {
      setIsSyncing(false);
    }
  }, [isAuthenticated]);

  const modeBadge = useMemo(
    () =>
      resolveCloudModeBadge({
        isAuthenticated,
        isOnline,
      }),
    [isAuthenticated, isOnline],
  );

  const value = useMemo(
    () => ({
      status: !isAuthenticated && status !== "offline" ? "unsigned" : status,
      lastSyncedAt,
      lastResult,
      isSyncing,
      modeBadge,
      syncNow,
      refreshConnectivity,
    }),
    [
      status,
      lastSyncedAt,
      lastResult,
      isSyncing,
      modeBadge,
      syncNow,
      refreshConnectivity,
      isAuthenticated,
    ],
  );

  return (
    <CloudSyncContext.Provider value={value}>{children}</CloudSyncContext.Provider>
  );
}

export function useCloudSync() {
  const context = useContext(CloudSyncContext);
  if (!context) {
    throw new Error("useCloudSync must be used within CloudSyncProvider");
  }
  return context;
}
