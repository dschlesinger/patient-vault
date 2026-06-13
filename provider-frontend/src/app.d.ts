// See https://svelte.dev/docs/kit/types#app.d.ts
import type { Session, SupabaseClient } from '@supabase/supabase-js';
import type { Database } from '$lib/types/database';

declare global {
	namespace App {
		interface Locals {
			supabase: SupabaseClient<Database>;
			safeGetSession: () => Promise<{ session: Session | null; user: import('@supabase/supabase-js').User | null }>;
		}
		interface PageData {
			session: Session | null;
		}
	}
}

export {};
