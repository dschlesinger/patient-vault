import { createSupabaseServerClient } from '$lib/supabase/server';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ cookies }) => {
  const supabase = createSupabaseServerClient(cookies);
  const { data: { user } } = await supabase.auth.getUser();
  return { user };
};
