import { fail, redirect } from '@sveltejs/kit';
import { createSupabaseServerClient } from '$lib/supabase/server';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import type { Actions, PageServerLoad } from './$types';

const MOCK_MODE = PUBLIC_SUPABASE_URL.includes('placeholder');

export const load: PageServerLoad = async ({ cookies }) => {
  if (MOCK_MODE) return {};

  const supabase = createSupabaseServerClient(cookies);
  const { data: { session } } = await supabase.auth.getSession();
  if (session) redirect(303, '/dashboard');
  return {};
};

export const actions: Actions = {
  default: async ({ request, cookies }) => {
    if (MOCK_MODE) redirect(303, '/dashboard');

    const supabase = createSupabaseServerClient(cookies);
    const data = await request.formData();
    const email = data.get('email') as string;
    const password = data.get('password') as string;

    const { error } = await supabase.auth.signInWithPassword({ email, password });
    if (error) return fail(400, { error: 'Invalid email or password.' });
    redirect(303, '/dashboard');
  }
};
