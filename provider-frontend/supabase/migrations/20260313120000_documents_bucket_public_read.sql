-- Make the documents bucket publicly readable so the USB app can download
-- encrypted files without Supabase auth (encryption is the access control).

update storage.buckets
set public = true
where id = 'documents';

drop policy if exists "documents_provider_select" on storage.objects;

create policy "documents_public_read"
  on storage.objects for select
  using (bucket_id = 'documents');
