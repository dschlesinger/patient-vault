<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import { getContext, onMount } from 'svelte';
  import type { SupabaseClient } from '@supabase/supabase-js';
  import { goto } from '$app/navigation';

  let { children } = $props();

  const supabase = getContext<SupabaseClient>('supabase');

  async function signOut() {
    await supabase.auth.signOut();
    goto('/');
  }

  let dark = $state(false);

  onMount(() => {
    dark = document.documentElement.classList.contains('dark');
  });

  function toggleDark() {
    dark = !dark;
    document.documentElement.classList.toggle('dark', dark);
    try { localStorage.setItem('theme', dark ? 'dark' : 'light'); } catch {}
  }
</script>

<div class="min-h-screen bg-[--color-nb-white] flex">
  <!-- Sidebar -->
  <nav class="w-64 bg-[--color-nb-surface] nb-border-thick border-r border-l-0 border-t-0 border-b-0 p-6 flex flex-col gap-4 shrink-0">
    <div class="mb-4">
      <h1 class="text-xl font-black tracking-tight">PatientVault</h1>
      <p class="text-xs font-medium text-[--color-nb-text-secondary] mt-0.5">Provider Portal</p>
    </div>

    <a
      href="/dashboard"
      class="font-bold px-3 py-2 nb-border nb-shadow-hover hover:bg-[--color-nb-accent] transition-colors"
    >
      Dashboard
    </a>

    <div class="mt-auto flex flex-col gap-2">
      <button
        onclick={toggleDark}
        class="font-bold px-3 py-2 nb-border nb-shadow-hover text-left transition-colors hover:bg-[--color-nb-muted]"
      >
        {dark ? '☀ Light mode' : '☾ Dark mode'}
      </button>
      <Button variant="secondary" class="w-full" onclick={signOut}>Sign out</Button>
    </div>
  </nav>

  <!-- Main content -->
  <main class="flex-1 p-8 overflow-auto">
    {@render children()}
  </main>
</div>
