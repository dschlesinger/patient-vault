import { fail } from '@sveltejs/kit';
import { createSupabaseServerClient } from '$lib/supabase/server';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import { MOCK_PATIENTS } from '$lib/mock/data';
import type { Actions, PageServerLoad } from './$types';

const MOCK_MODE = PUBLIC_SUPABASE_URL.includes('placeholder');

export const load: PageServerLoad = async ({ cookies }) => {
  if (MOCK_MODE) return { patients: MOCK_PATIENTS };

  const supabase = createSupabaseServerClient(cookies);
  const { data: { user } } = await supabase.auth.getUser();

  const { data: patients, error } = await supabase
    .from('patient_provider_links')
    .select('usb_id, patient_name, registered_at')
    .eq('provider_id', user!.id)
    .order('registered_at', { ascending: false });

  if (error) return { patients: [] };
  return { patients: patients ?? [] };
};

async function insertPairingCode(supabase: ReturnType<typeof createSupabaseServerClient>, providerId: string) {
  const code = String(Math.floor(Math.random() * 1_000_000)).padStart(6, '0');
  const expiresAt = new Date(Date.now() + 10 * 60 * 1000).toISOString();
  const { error } = await supabase.from('pairing_codes').insert({
    code,
    provider_id: providerId,
    expires_at: expiresAt,
    used: false
  });
  return { code, error };
}

export const actions: Actions = {
  generate_code: async ({ cookies }) => {
    if (MOCK_MODE) return { pairingCode: String(Math.floor(Math.random() * 1_000_000)).padStart(6, '0') };

    const supabase = createSupabaseServerClient(cookies);
    const { data: { user } } = await supabase.auth.getUser();
    if (!user) return fail(401, { error: 'Not authenticated.' });

    const { code, error } = await insertPairingCode(supabase, user.id);
    if (error) return fail(500, { error: 'Failed to generate pairing code.' });
    return { pairingCode: code };
  },

  refresh_code: async ({ cookies, request }) => {
    if (MOCK_MODE) return { pairingCode: String(Math.floor(Math.random() * 1_000_000)).padStart(6, '0') };

    const supabase = createSupabaseServerClient(cookies);
    const { data: { user } } = await supabase.auth.getUser();
    if (!user) return fail(401, { error: 'Not authenticated.' });

    const formData = await request.formData();
    const oldCode = formData.get('old_code') as string | null;

    // Expire the old code immediately so it can't be used
    if (oldCode) {
      await supabase
        .from('pairing_codes')
        .update({ used: true })
        .eq('code', oldCode)
        .eq('provider_id', user.id);
    }

    const { code, error } = await insertPairingCode(supabase, user.id);
    if (error) return fail(500, { error: 'Failed to refresh pairing code.' });
    return { pairingCode: code };
  },

  cancel_code: async ({ cookies, request }) => {
    if (MOCK_MODE) return { pairingCode: null };

    const supabase = createSupabaseServerClient(cookies);
    const { data: { user } } = await supabase.auth.getUser();
    if (!user) return fail(401, { error: 'Not authenticated.' });

    const formData = await request.formData();
    const oldCode = formData.get('old_code') as string | null;

    if (oldCode) {
      await supabase
        .from('pairing_codes')
        .update({ used: true })
        .eq('code', oldCode)
        .eq('provider_id', user.id);
    }

    return { pairingCode: null };
  }
};
