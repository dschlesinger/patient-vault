import { redirect } from '@sveltejs/kit';
import { createSupabaseServerClient } from '$lib/supabase/server';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import type { LayoutServerLoad } from './$types';

const MOCK_MODE = PUBLIC_SUPABASE_URL.includes('placeholder');

export const load: LayoutServerLoad = async ({ cookies }) => {
  if (MOCK_MODE) return { session: null };

  const supabase = createSupabaseServerClient(cookies);
  const { data: { user } } = await supabase.auth.getUser();
  if (!user) redirect(303, '/');
  return {};
};
