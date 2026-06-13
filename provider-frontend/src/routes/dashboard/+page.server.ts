import { fail } from '@sveltejs/kit';
import { createSupabaseServerClient } from '$lib/supabase/server';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import { MOCK_PATIENTS } from '$lib/mock/data';
import type { Actions, PageServerLoad } from './$types';

const MOCK_MODE = PUBLIC_SUPABASE_URL.includes('placeholder');

export const load: PageServerLoad = async ({ cookies }) => {
  if (MOCK_MODE) return { patients: MOCK_PATIENTS };

  const supabase = createSupabaseServerClient(cookies);
  const { data: { session } } = await supabase.auth.getSession();

  const { data: patients, error } = await supabase
    .from('patient_provider_links')
    .select('usb_id, patient_name, registered_at')
    .eq('provider_id', session!.user.id)
    .order('registered_at', { ascending: false });

  if (error) return { patients: [] };
  return { patients: patients ?? [] };
};

export const actions: Actions = {
  generate_code: async ({ cookies }) => {
    if (MOCK_MODE) return { pairingCode: '482917' };

    const supabase = createSupabaseServerClient(cookies);
    const { data: { session } } = await supabase.auth.getSession();
    if (!session) return fail(401, { error: 'Not authenticated.' });

    const code = String(Math.floor(Math.random() * 1_000_000)).padStart(6, '0');
    const expiresAt = new Date(Date.now() + 10 * 60 * 1000).toISOString();

    const { error } = await supabase.from('pairing_codes').insert({
      code,
      provider_id: session.user.id,
      expires_at: expiresAt,
      used: false
    });

    if (error) return fail(500, { error: 'Failed to generate pairing code.' });
    return { pairingCode: code };
  }
};
