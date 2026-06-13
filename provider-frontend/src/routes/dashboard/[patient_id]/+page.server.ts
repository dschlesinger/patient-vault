import { fail, error } from '@sveltejs/kit';
import { createSupabaseServerClient } from '$lib/supabase/server';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import { MOCK_PATIENTS_FULL, MOCK_SENT_LOG } from '$lib/mock/data';
import type { Actions, PageServerLoad } from './$types';

const MOCK_MODE = PUBLIC_SUPABASE_URL.includes('placeholder');

export const load: PageServerLoad = async ({ cookies, params }) => {
  if (MOCK_MODE) {
    const patient = MOCK_PATIENTS_FULL.find((p) => p.usb_id === params.patient_id);
    if (!patient) error(404, 'Patient not found');
    return { patient, sentLog: MOCK_SENT_LOG[params.patient_id] ?? [] };
  }

  const supabase = createSupabaseServerClient(cookies);
  const { data: { session } } = await supabase.auth.getSession();

  const { data: patient } = await supabase
    .from('patient_provider_links')
    .select('usb_id, patient_name, registered_at, public_key')
    .eq('usb_id', params.patient_id)
    .eq('provider_id', session!.user.id)
    .single();

  if (!patient) error(404, 'Patient not found');

  const { data: sentLog } = await supabase
    .from('provider_sent_log')
    .select('id, type, sent_at, payload_ref_id')
    .eq('usb_id', params.patient_id)
    .eq('provider_id', session!.user.id)
    .order('sent_at', { ascending: false });

  return { patient, sentLog: sentLog ?? [] };
};

export const actions: Actions = {
  send_questionnaire: async ({ cookies, params, request }) => {
    if (MOCK_MODE) return { success: true };

    const supabase = createSupabaseServerClient(cookies);
    const { data: { session } } = await supabase.auth.getSession();
    const formData = await request.formData();
    const encryptedBlob = formData.get('encrypted_blob') as string;
    if (!encryptedBlob) return fail(400, { error: 'Missing encrypted payload.' });

    const { data: payload, error: insertError } = await supabase
      .from('payloads')
      .insert({ usb_id: params.patient_id, type: 'questionnaire', encrypted_blob: encryptedBlob })
      .select('id')
      .single();

    if (insertError || !payload) return fail(500, { error: 'Failed to store questionnaire.' });

    await supabase.from('provider_sent_log').insert({
      provider_id: session!.user.id,
      usb_id: params.patient_id,
      type: 'questionnaire',
      payload_ref_id: payload.id
    });

    return { success: true };
  },

  send_document: async ({ cookies, params, request }) => {
    if (MOCK_MODE) return { success: true };

    const supabase = createSupabaseServerClient(cookies);
    const { data: { session } } = await supabase.auth.getSession();
    const formData = await request.formData();
    const encryptedFile = formData.get('encrypted_file') as File;
    if (!encryptedFile) return fail(400, { error: 'Missing encrypted file.' });

    const { data: payload, error: insertError } = await supabase
      .from('payloads')
      .insert({ usb_id: params.patient_id, type: 'document', encrypted_blob: null })
      .select('id')
      .single();

    if (insertError || !payload) return fail(500, { error: 'Failed to create document payload.' });

    const storagePath = `documents/${payload.id}`;
    const fileBytes = await encryptedFile.arrayBuffer();

    const { error: uploadError } = await supabase.storage
      .from('documents')
      .upload(storagePath, fileBytes, { contentType: 'application/octet-stream' });

    if (uploadError) return fail(500, { error: 'Failed to upload document.' });

    await supabase.from('payloads').update({ storage_path: storagePath }).eq('id', payload.id);
    await supabase.from('provider_sent_log').insert({
      provider_id: session!.user.id,
      usb_id: params.patient_id,
      type: 'document',
      payload_ref_id: payload.id
    });

    return { success: true };
  }
};
