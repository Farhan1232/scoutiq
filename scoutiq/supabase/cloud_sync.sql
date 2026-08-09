-- =============================================================================
-- ScoutIQ Cloud Sync schema (safe to re-run)
-- Run this in the Supabase SQL Editor BEFORE using cloud sync in the app.
--
-- Domains:
--   1) profiles            (already used by auth — kept / aligned here)
--   2) app_settings
--   3) save_metadata       (metadata ONLY — never full .fm save blobs)
--   4) analysis_history
--   5) transfer_shortlist
--
-- Every row is scoped to auth.uid() via RLS.
-- =============================================================================

-- ---------------------------------------------------------------------------
-- 1) Profiles
-- ---------------------------------------------------------------------------
create table if not exists public.profiles (
  user_id uuid primary key references auth.users (id) on delete cascade,
  display_name text not null,
  language text not null default 'de' check (language in ('de', 'en')),
  theme text not null default 'dark' check (theme in ('dark', 'light', 'system')),
  plan text not null default 'free' check (plan in ('free', 'pro', 'elite')),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

alter table public.profiles enable row level security;

drop policy if exists "Profiles are readable by owner" on public.profiles;
create policy "Profiles are readable by owner"
  on public.profiles for select
  using (auth.uid() = user_id);

drop policy if exists "Profiles are insertable by owner" on public.profiles;
create policy "Profiles are insertable by owner"
  on public.profiles for insert
  with check (auth.uid() = user_id);

drop policy if exists "Profiles are updatable by owner" on public.profiles;
create policy "Profiles are updatable by owner"
  on public.profiles for update
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

drop policy if exists "Profiles are deletable by owner" on public.profiles;
create policy "Profiles are deletable by owner"
  on public.profiles for delete
  using (auth.uid() = user_id);

create or replace function public.handle_new_user()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  insert into public.profiles (user_id, display_name, language, theme, plan)
  values (
    new.id,
    coalesce(new.raw_user_meta_data->>'display_name', split_part(new.email, '@', 1), 'ScoutIQ'),
    'de',
    'dark',
    'free'
  )
  on conflict (user_id) do nothing;
  return new;
end;
$$;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_user();

-- ---------------------------------------------------------------------------
-- 2) App settings
-- ---------------------------------------------------------------------------
create table if not exists public.app_settings (
  user_id uuid primary key references auth.users (id) on delete cascade,
  theme_preference text not null default 'dark'
    check (theme_preference in ('dark', 'light', 'system')),
  language text not null default 'de' check (language in ('de', 'en')),
  notifications_enabled boolean not null default true,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

alter table public.app_settings enable row level security;

drop policy if exists "App settings readable by owner" on public.app_settings;
create policy "App settings readable by owner"
  on public.app_settings for select
  using (auth.uid() = user_id);

drop policy if exists "App settings insertable by owner" on public.app_settings;
create policy "App settings insertable by owner"
  on public.app_settings for insert
  with check (auth.uid() = user_id);

drop policy if exists "App settings updatable by owner" on public.app_settings;
create policy "App settings updatable by owner"
  on public.app_settings for update
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

drop policy if exists "App settings deletable by owner" on public.app_settings;
create policy "App settings deletable by owner"
  on public.app_settings for delete
  using (auth.uid() = user_id);

-- ---------------------------------------------------------------------------
-- 3) Save metadata (NO full Football Manager save files)
-- ---------------------------------------------------------------------------
create table if not exists public.save_metadata (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users (id) on delete cascade,
  save_name text not null,
  club_name text not null,
  competition text not null,
  season text not null,
  game_date date not null,
  last_opened_at timestamptz not null default now(),
  file_name text not null,
  file_size_bytes bigint,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create index if not exists save_metadata_user_id_idx
  on public.save_metadata (user_id);

create index if not exists save_metadata_user_updated_idx
  on public.save_metadata (user_id, updated_at desc);

alter table public.save_metadata enable row level security;

drop policy if exists "Save metadata readable by owner" on public.save_metadata;
create policy "Save metadata readable by owner"
  on public.save_metadata for select
  using (auth.uid() = user_id);

drop policy if exists "Save metadata insertable by owner" on public.save_metadata;
create policy "Save metadata insertable by owner"
  on public.save_metadata for insert
  with check (auth.uid() = user_id);

drop policy if exists "Save metadata updatable by owner" on public.save_metadata;
create policy "Save metadata updatable by owner"
  on public.save_metadata for update
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

drop policy if exists "Save metadata deletable by owner" on public.save_metadata;
create policy "Save metadata deletable by owner"
  on public.save_metadata for delete
  using (auth.uid() = user_id);

-- ---------------------------------------------------------------------------
-- 4) Analysis history
-- ---------------------------------------------------------------------------
create table if not exists public.analysis_history (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users (id) on delete cascade,
  title text not null,
  category text not null
    check (category in ('tactics', 'transfer', 'opponent', 'training', 'other')),
  summary text not null default '',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create index if not exists analysis_history_user_id_idx
  on public.analysis_history (user_id);

create index if not exists analysis_history_user_updated_idx
  on public.analysis_history (user_id, updated_at desc);

alter table public.analysis_history enable row level security;

drop policy if exists "Analysis history readable by owner" on public.analysis_history;
create policy "Analysis history readable by owner"
  on public.analysis_history for select
  using (auth.uid() = user_id);

drop policy if exists "Analysis history insertable by owner" on public.analysis_history;
create policy "Analysis history insertable by owner"
  on public.analysis_history for insert
  with check (auth.uid() = user_id);

drop policy if exists "Analysis history updatable by owner" on public.analysis_history;
create policy "Analysis history updatable by owner"
  on public.analysis_history for update
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

drop policy if exists "Analysis history deletable by owner" on public.analysis_history;
create policy "Analysis history deletable by owner"
  on public.analysis_history for delete
  using (auth.uid() = user_id);

-- ---------------------------------------------------------------------------
-- 5) Transfer shortlist
-- ---------------------------------------------------------------------------
create table if not exists public.transfer_shortlist (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users (id) on delete cascade,
  player_name text not null,
  position text not null,
  club text not null,
  age integer not null check (age >= 14 and age <= 60),
  estimated_value text not null default '',
  priority text not null default 'medium'
    check (priority in ('high', 'medium', 'low')),
  notes text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  deleted_at timestamptz
);

create index if not exists transfer_shortlist_user_id_idx
  on public.transfer_shortlist (user_id);

create index if not exists transfer_shortlist_user_updated_idx
  on public.transfer_shortlist (user_id, updated_at desc);

alter table public.transfer_shortlist enable row level security;

drop policy if exists "Transfer shortlist readable by owner" on public.transfer_shortlist;
create policy "Transfer shortlist readable by owner"
  on public.transfer_shortlist for select
  using (auth.uid() = user_id);

drop policy if exists "Transfer shortlist insertable by owner" on public.transfer_shortlist;
create policy "Transfer shortlist insertable by owner"
  on public.transfer_shortlist for insert
  with check (auth.uid() = user_id);

drop policy if exists "Transfer shortlist updatable by owner" on public.transfer_shortlist;
create policy "Transfer shortlist updatable by owner"
  on public.transfer_shortlist for update
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

drop policy if exists "Transfer shortlist deletable by owner" on public.transfer_shortlist;
create policy "Transfer shortlist deletable by owner"
  on public.transfer_shortlist for delete
  using (auth.uid() = user_id);
