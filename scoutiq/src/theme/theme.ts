export const THEME_PREFERENCES = ["dark", "light", "system"] as const;
export type ThemePreference = (typeof THEME_PREFERENCES)[number];

export const RESOLVED_THEMES = ["dark", "light"] as const;
export type ResolvedTheme = (typeof RESOLVED_THEMES)[number];

export const THEME_STORAGE_KEY = "scoutiq.theme";
export const DEFAULT_THEME_PREFERENCE: ThemePreference = "dark";

export function isThemePreference(value: string): value is ThemePreference {
  return (THEME_PREFERENCES as readonly string[]).includes(value);
}

export function getSystemTheme(): ResolvedTheme {
  if (typeof window === "undefined" || !window.matchMedia) {
    return "dark";
  }

  return window.matchMedia("(prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
}

export function resolveTheme(preference: ThemePreference): ResolvedTheme {
  return preference === "system" ? getSystemTheme() : preference;
}

export function readStoredThemePreference(): ThemePreference {
  try {
    const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (stored && isThemePreference(stored)) {
      return stored;
    }
  } catch {
    // localStorage can be unavailable in restricted environments
  }

  return DEFAULT_THEME_PREFERENCE;
}

export function persistThemePreference(preference: ThemePreference) {
  try {
    window.localStorage.setItem(THEME_STORAGE_KEY, preference);
  } catch {
    // Ignore persistence failures; runtime theme still applies.
  }
}

export function applyResolvedTheme(theme: ResolvedTheme) {
  const root = document.documentElement;
  root.setAttribute("data-theme", theme);
  root.style.colorScheme = theme;
}

export function applyThemePreference(preference: ThemePreference) {
  const resolved = resolveTheme(preference);
  document.documentElement.setAttribute("data-theme-preference", preference);
  applyResolvedTheme(resolved);
  return resolved;
}
