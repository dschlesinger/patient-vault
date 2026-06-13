-- Patient pairing RPC: validates a 6-digit code and links usb_id + public_key to a provider.
-- Callable by anon (USB app has no Supabase auth). SECURITY DEFINER bypasses RLS.

-- Multi-provider: one usb_id may link to many providers (drop single-column unique).
alter table public.patient_provider_links
  drop constraint if exists patient_provider_links_usb_id_key;

create unique index if not exists patient_provider_links_usb_provider_idx
  on public.patient_provider_links (usb_id, provider_id);

create or replace function public.register_patient(
  p_usb_id text,
  p_public_key text,
  p_patient_name text,
  p_provider_code text
)
returns jsonb
language plpgsql
security definer
set search_path = public
as $$
declare
  v_provider_id uuid;
  v_code_id uuid;
begin
  if coalesce(trim(p_usb_id), '') = '' then
    return jsonb_build_object('ok', false, 'error', 'usb_id is required');
  end if;
  if coalesce(trim(p_public_key), '') = '' then
    return jsonb_build_object('ok', false, 'error', 'public_key is required');
  end if;
  if coalesce(trim(p_patient_name), '') = '' then
    return jsonb_build_object('ok', false, 'error', 'patient_name is required');
  end if;
  if length(trim(p_provider_code)) <> 6 or trim(p_provider_code) !~ '^\d{6}$' then
    return jsonb_build_object('ok', false, 'error', 'Pairing code must be 6 digits');
  end if;

  select id, provider_id
  into v_code_id, v_provider_id
  from public.pairing_codes
  where code = trim(p_provider_code)
    and used = false
    and expires_at > now()
  order by created_at desc
  limit 1;

  if v_provider_id is null then
    return jsonb_build_object('ok', false, 'error', 'Invalid or expired pairing code.');
  end if;

  insert into public.patient_provider_links (usb_id, public_key, patient_name, provider_id)
  values (trim(p_usb_id), trim(p_public_key), trim(p_patient_name), v_provider_id)
  on conflict (usb_id, provider_id) do update
    set public_key = excluded.public_key,
        patient_name = excluded.patient_name,
        registered_at = now();

  update public.pairing_codes
  set used = true
  where id = v_code_id;

  return jsonb_build_object('ok', true, 'provider_id', v_provider_id);
end;
$$;

revoke all on function public.register_patient(text, text, text, text) from public;
grant execute on function public.register_patient(text, text, text, text) to anon, authenticated, service_role;
