-- payloads table is publicly readable — encryption is the access control.
-- Required for INSERT ... RETURNING (providers) and USB app polling (no auth).

create policy "payloads_public_read"
  on public.payloads for select
  using (true);
