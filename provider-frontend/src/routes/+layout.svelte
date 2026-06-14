<script lang="ts">
  import '../app.css';
  import { setContext } from 'svelte';
  import { createSupabaseBrowserClient } from '$lib/supabase/client';

  let { data, children } = $props();

  const supabase = createSupabaseBrowserClient();
  setContext('supabase', supabase);
</script>

<svelte:head>
  <title>PatientVault — Provider</title>
</svelte:head>

{@render children()}
