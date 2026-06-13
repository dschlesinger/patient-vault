// See https://svelte.dev/docs/kit/types#app.d.ts
import type { SupabaseClient, User } from '@supabase/supabase-js';
import type { Database } from '$lib/types/database';

declare global {
	namespace App {
		interface Locals {
			supabase: SupabaseClient<Database>;
		}
		interface PageData {
			user: User | null;
		}
	}
}

export {};
