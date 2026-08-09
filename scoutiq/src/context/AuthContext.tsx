import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { setAppLanguage } from "../i18n/i18n";
import * as authService from "../services/authService";
import type {
  AuthResult,
  AuthSession,
  AuthUser,
  EmailPasswordCredentials,
  ProfileUpdateInput,
  SignUpCredentials,
} from "../auth/types";

interface AuthContextValue {
  user: AuthUser | null;
  session: AuthSession | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  signInWithPassword: (
    credentials: EmailPasswordCredentials,
  ) => Promise<AuthResult>;
  signUp: (credentials: SignUpCredentials) => Promise<AuthResult>;
  resetPassword: (email: string) => Promise<AuthResult>;
  updateProfile: (input: ProfileUpdateInput) => Promise<AuthResult>;
  signOut: () => Promise<void>;
  refreshSession: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

interface AuthProviderProps {
  children: ReactNode;
}

export function AuthProvider({ children }: AuthProviderProps) {
  const [session, setSession] = useState<AuthSession | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const applySession = useCallback(async (next: AuthSession | null) => {
    setSession(next);
    if (next?.user.language) {
      await setAppLanguage(next.user.language);
    }
  }, []);

  const refreshSession = useCallback(async () => {
    const next = await authService.getSession();
    await applySession(next);
  }, [applySession]);

  const applyResult = useCallback(
    async (result: AuthResult) => {
      if (result.ok && result.session) {
        await applySession(result.session);
      }
      return result;
    },
    [applySession],
  );

  useEffect(() => {
    let active = true;

    void (async () => {
      try {
        const next = await authService.getSession();
        if (active) {
          await applySession(next);
        }
      } finally {
        if (active) {
          setIsLoading(false);
        }
      }
    })();

    const unsubscribe = authService.onAuthStateChange((next) => {
      if (!active) return;
      void applySession(next);
    });

    return () => {
      active = false;
      unsubscribe();
    };
  }, [applySession]);

  const signInWithPassword = useCallback(
    async (credentials: EmailPasswordCredentials) => {
      const result = await authService.signInWithPassword(credentials);
      return applyResult(result);
    },
    [applyResult],
  );

  const signUp = useCallback(
    async (credentials: SignUpCredentials) => {
      const result = await authService.signUp(credentials);
      return applyResult(result);
    },
    [applyResult],
  );

  const resetPassword = useCallback(async (email: string) => {
    return authService.resetPassword(email);
  }, []);

  const updateProfile = useCallback(
    async (input: ProfileUpdateInput) => {
      const result = await authService.updateProfile(input);
      return applyResult(result);
    },
    [applyResult],
  );

  const signOut = useCallback(async () => {
    await authService.signOut();
    setSession(null);
  }, []);

  const value = useMemo<AuthContextValue>(
    () => ({
      user: session?.user ?? null,
      session,
      isAuthenticated: Boolean(session?.user),
      isLoading,
      signInWithPassword,
      signUp,
      resetPassword,
      updateProfile,
      signOut,
      refreshSession,
    }),
    [
      session,
      isLoading,
      signInWithPassword,
      signUp,
      resetPassword,
      updateProfile,
      signOut,
      refreshSession,
    ],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within AuthProvider");
  }
  return context;
}
