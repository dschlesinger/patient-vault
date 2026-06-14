import { fail, redirect } from '@sveltejs/kit';
import { createSupabaseServerClient } from '$lib/supabase/server';
import { PUBLIC_SUPABASE_URL } from '$env/static/public';
import type { Actions, PageServerLoad } from './$types';

const MOCK_MODE = PUBLIC_SUPABASE_URL.includes('placeholder');

export const load: PageServerLoad = async ({ cookies }) => {
  if (MOCK_MODE) return {};

  const supabase = createSupabaseServerClient(cookies);
  const { data: { user } } = await supabase.auth.getUser();
  if (user) redirect(303, '/dashboard');
  return {};
};

export const actions: Actions = {
  signin: async ({ request, cookies }) => {
    if (MOCK_MODE) redirect(303, '/dashboard');

    const supabase = createSupabaseServerClient(cookies);
    const data = await request.formData();
    const email = data.get('email') as string;
    const password = data.get('password') as string;

    const { error } = await supabase.auth.signInWithPassword({ email, password });
    if (error) return fail(400, { error: 'Invalid email or password.' });
    redirect(303, '/dashboard');
  },

  signup: async ({ request, cookies }) => {
    if (MOCK_MODE) redirect(303, '/dashboard');

    const supabase = createSupabaseServerClient(cookies);
    const data = await request.formData();
    const email = data.get('email') as string;
    const password = data.get('password') as string;
    const name = (data.get('name') as string | null)?.trim() ?? '';

    if (!email || !password) {
      return fail(400, { error: 'Email and password are required.' });
    }
    if (password.length < 8) {
      return fail(400, { error: 'Password must be at least 8 characters.' });
    }

    const { error } = await supabase.auth.signUp({
      email,
      password,
      options: { data: { name } }
    });
    if (error) return fail(400, { error: error.message });
    redirect(303, '/dashboard');
  }
};
