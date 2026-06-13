-- PatientVault initial schema
-- Run via Supabase CLI (`supabase db push`) or paste into the SQL editor.

-- ── Enum ──────────────────────────────────────────────────────────────────────
create type public.payload_type as enum ('message', 'questionnaire', 'document');

-- ── Tables ────────────────────────────────────────────────────────────────────
create table public.providers (
  id uuid primary key references auth.users (id) on delete cascade,
  name text not null default '',
  email text not null,
  created_at timestamptz not null default now()
);

create table public.pairing_codes (
  id uuid primary key default gen_random_uuid(),
  code text not null,
  provider_id uuid not null references public.providers (id) on delete cascade,
  expires_at timestamptz not null,
  used boolean not null default false,
  created_at timestamptz not null default now()
);

create unique index pairing_codes_active_code_idx
  on public.pairing_codes (code)
  where used = false;

create table public.patient_provider_links (
  id uuid primary key default gen_random_uuid(),
  usb_id text not null unique,
  public_key text not null,
  patient_name text not null,
  provider_id uuid not null references public.providers (id) on delete cascade,
  registered_at timestamptz not null default now()
);

create table public.payloads (
  id uuid primary key default gen_random_uuid(),
  usb_id text not null,
  type public.payload_type not null,
  encrypted_blob text,
  storage_path text,
  created_at timestamptz not null default now()
);

create table public.provider_sent_log (
  id uuid primary key default gen_random_uuid(),
  provider_id uuid not null references public.providers (id) on delete cascade,
  usb_id text not null,
  type public.payload_type not null,
  sent_at timestamptz not null default now(),
  payload_ref_id uuid not null references public.payloads (id) on delete cascade
);

-- ── Auth trigger: auto-create providers row on sign-up ────────────────────────
create or replace function public.handle_new_provider()
returns trigger
language plpgsql
security definer
set search_path = public
as $$
begin
  insert into public.providers (id, email, name)
  values (new.id, new.email, coalesce(new.raw_user_meta_data ->> 'name', ''));
  return new;
end;
$$;

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function public.handle_new_provider();

-- ── Row Level Security ────────────────────────────────────────────────────────
alter table public.providers enable row level security;
alter table public.pairing_codes enable row level security;
alter table public.patient_provider_links enable row level security;
alter table public.payloads enable row level security;
alter table public.provider_sent_log enable row level security;

-- providers: own row only
create policy "providers_select_own"
  on public.providers for select
  using (id = auth.uid());

create policy "providers_update_own"
  on public.providers for update
  using (id = auth.uid());

-- pairing_codes: provider manages own codes
create policy "pairing_codes_select_own"
  on public.pairing_codes for select
  using (provider_id = auth.uid());

create policy "pairing_codes_insert_own"
  on public.pairing_codes for insert
  with check (provider_id = auth.uid());

create policy "pairing_codes_update_own"
  on public.pairing_codes for update
  using (provider_id = auth.uid());

-- patient_provider_links: provider sees own patients
create policy "patient_links_select_own"
  on public.patient_provider_links for select
  using (provider_id = auth.uid());

create policy "patient_links_insert_open"
  on public.patient_provider_links for insert
  with check (true);

-- payloads: providers insert for linked patients; public read added in follow-up migration
create policy "providers_insert_payloads"
  on public.payloads for insert
  with check (
    auth.role() = 'authenticated'
    and exists (
      select 1 from public.patient_provider_links
      where usb_id = payloads.usb_id
        and provider_id = auth.uid()
    )
  );

create policy "providers_update_payloads"
  on public.payloads for update
  using (
    auth.role() = 'authenticated'
    and exists (
      select 1 from public.patient_provider_links
      where usb_id = payloads.usb_id
        and provider_id = auth.uid()
    )
  );

-- provider_sent_log: provider sees/inserts own entries
create policy "provider_own_sent_log_select"
  on public.provider_sent_log for select
  using (provider_id = auth.uid());

create policy "provider_own_sent_log_insert"
  on public.provider_sent_log for insert
  with check (provider_id = auth.uid());

-- ── Storage bucket (documents) ────────────────────────────────────────────────
insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values (
  'documents',
  'documents',
  false,
  52428800,
  array['application/octet-stream']::text[]
)
on conflict (id) do nothing;

create policy "documents_provider_insert"
  on storage.objects for insert
  with check (
    bucket_id = 'documents'
    and auth.uid() is not null
  );

create policy "documents_provider_select"
  on storage.objects for select
  using (
    bucket_id = 'documents'
    and auth.uid() is not null
  );
