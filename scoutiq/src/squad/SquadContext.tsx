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
import { importSquadFromFile } from "../import/importService";
import type { ImportError } from "../import/types";
import { clearSquad, loadSquad, saveSquad } from "./squadStore";
import type { Squad } from "./types";

interface SquadContextValue {
  squad: Squad | null;
  isImporting: boolean;
  lastError: ImportError | null;
  importSquad: () => Promise<boolean>;
  clearImportedSquad: () => void;
  clearError: () => void;
}

const SquadContext = createContext<SquadContextValue | null>(null);

export function SquadProvider({ children }: SquadProviderProps) {
  const { user } = useAuth();
  const [squad, setSquad] = useState<Squad | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [lastError, setLastError] = useState<ImportError | null>(null);

  useEffect(() => {
    if (!user?.id) {
      setSquad(null);
      return;
    }
    setSquad(loadSquad(user.id));
  }, [user?.id]);

  const importSquad = useCallback(async () => {
    if (!user?.id) return false;
    setIsImporting(true);
    setLastError(null);

    try {
      const result = await importSquadFromFile();
      if (!result.ok) {
        if (result.error.code !== "cancelled") {
          setLastError(result.error);
        }
        return false;
      }

      saveSquad(user.id, result.squad);
      setSquad(result.squad);
      return true;
    } finally {
      setIsImporting(false);
    }
  }, [user?.id]);

  const clearImportedSquad = useCallback(() => {
    if (!user?.id) return;
    clearSquad(user.id);
    setSquad(null);
    setLastError(null);
  }, [user?.id]);

  const clearError = useCallback(() => setLastError(null), []);

  const value = useMemo(
    () => ({
      squad,
      isImporting,
      lastError,
      importSquad,
      clearImportedSquad,
      clearError,
    }),
    [squad, isImporting, lastError, importSquad, clearImportedSquad, clearError],
  );

  return (
    <SquadContext.Provider value={value}>{children}</SquadContext.Provider>
  );
}

interface SquadProviderProps {
  children: ReactNode;
}

export function useSquad() {
  const context = useContext(SquadContext);
  if (!context) {
    throw new Error("useSquad must be used within SquadProvider");
  }
  return context;
}
