import { json, error } from '@sveltejs/kit';
import { createClient } from '@supabase/supabase-js';
import { PUBLIC_SUPABASE_URL, PUBLIC_SUPABASE_ANON_KEY } from '$env/static/public';
import type { RequestHandler } from './$types';

/** Open registration endpoint called by the USB app during in-person pairing. */
export const POST: RequestHandler = async ({ request }) => {
  let body: {
    usb_id?: string;
    public_key?: string;
    patient_name?: string;
    provider_code?: string;
  };

  try {
    body = await request.json();
  } catch {
    throw error(400, 'Invalid JSON body.');
  }

  const { usb_id, public_key, patient_name, provider_code } = body;
  if (!usb_id || !public_key || !patient_name || !provider_code) {
    throw error(400, 'Missing required fields: usb_id, public_key, patient_name, provider_code.');
  }

  const supabase = createClient(PUBLIC_SUPABASE_URL, PUBLIC_SUPABASE_ANON_KEY);
  const { data, error: rpcError } = await supabase.rpc('register_patient', {
    p_usb_id: usb_id,
    p_public_key: public_key,
    p_patient_name: patient_name,
    p_provider_code: provider_code
  });

  if (rpcError) {
    throw error(500, rpcError.message);
  }

  const result = data as { ok?: boolean; error?: string; provider_id?: string };
  if (!result?.ok) {
    return json({ ok: false, error: result?.error ?? 'Registration failed.' }, { status: 400 });
  }

  return json({ ok: true, provider_id: result.provider_id });
};
