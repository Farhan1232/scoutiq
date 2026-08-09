import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  DEFAULT_THEME_PREFERENCE,
  applyThemePreference,
  persistThemePreference,
  readStoredThemePreference,
  resolveTheme,
  type ResolvedTheme,
  type ThemePreference,
} from "./theme";

interface ThemeContextValue {
  preference: ThemePreference;
  resolvedTheme: ResolvedTheme;
  setPreference: (preference: ThemePreference) => void;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

interface ThemeProviderProps {
  children: ReactNode;
}

function getInitialPreference(): ThemePreference {
  return typeof window === "undefined"
    ? DEFAULT_THEME_PREFERENCE
    : readStoredThemePreference();
}

export function ThemeProvider({ children }: ThemeProviderProps) {
  const [preference, setPreferenceState] = useState<ThemePreference>(
    getInitialPreference,
  );
  const [resolvedTheme, setResolvedTheme] = useState<ResolvedTheme>(() =>
    resolveTheme(getInitialPreference()),
  );

  const setPreference = useCallback((next: ThemePreference) => {
    persistThemePreference(next);
    const resolved = applyThemePreference(next);
    setPreferenceState(next);
    setResolvedTheme(resolved);
  }, []);

  useEffect(() => {
    const resolved = applyThemePreference(preference);
    setResolvedTheme(resolved);
  }, [preference]);

  useEffect(() => {
    if (preference !== "system") {
      return;
    }

    const media = window.matchMedia("(prefers-color-scheme: light)");
    const onChange = () => {
      setResolvedTheme(applyThemePreference("system"));
    };

    media.addEventListener("change", onChange);
    return () => media.removeEventListener("change", onChange);
  }, [preference]);

  const value = useMemo(
    () => ({
      preference,
      resolvedTheme,
      setPreference,
    }),
    [preference, resolvedTheme, setPreference],
  );

  return (
    <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>
  );
}

export function useTheme() {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error("useTheme must be used within ThemeProvider");
  }
  return context;
}
