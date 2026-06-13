import { fail, redirect } from '@sveltejs/kit';
import { createSupabaseServerClient } from '$lib/supabase/server';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ cookies }) => {
  const supabase = createSupabaseServerClient(cookies);
  const {
    data: { session }
  } = await supabase.auth.getSession();

  if (session) {
    redirect(303, '/dashboard');
  }

  return {};
};

export const actions: Actions = {
  default: async ({ request, cookies }) => {
    const supabase = createSupabaseServerClient(cookies);
    const data = await request.formData();
    const email = data.get('email') as string;
    const password = data.get('password') as string;

    const { error } = await supabase.auth.signInWithPassword({ email, password });

    if (error) {
      return fail(400, { error: 'Invalid email or password.' });
    }

    redirect(303, '/dashboard');
  }
};
