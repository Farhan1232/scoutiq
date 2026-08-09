/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_SUPABASE_URL?: string;
  readonly VITE_SUPABASE_PUBLISHABLE_KEY?: string;
  /** @deprecated Prefer VITE_SUPABASE_PUBLISHABLE_KEY */
  readonly VITE_SUPABASE_ANON_KEY?: string;
  readonly VITE_SUPABASE_REDIRECT_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
