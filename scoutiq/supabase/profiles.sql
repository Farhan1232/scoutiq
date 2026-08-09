-- ScoutIQ profiles table (safe to re-run in Supabase SQL editor)
-- Integrates with auth.users via user_id = auth.uid()

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
  on public.profiles
  for select
  using (auth.uid() = user_id);

drop policy if exists "Profiles are insertable by owner" on public.profiles;
create policy "Profiles are insertable by owner"
  on public.profiles
  for insert
  with check (auth.uid() = user_id);

drop policy if exists "Profiles are updatable by owner" on public.profiles;
create policy "Profiles are updatable by owner"
  on public.profiles
  for update
  using (auth.uid() = user_id)
  with check (auth.uid() = user_id);

-- Optional: auto-create profile on signup
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
