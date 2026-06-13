<script lang="ts">
  import '../app.css';
  import BrowserDevBanner from '$lib/components/BrowserDevBanner.svelte';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';

  let { children } = $props();
</script>

<svelte:head>
  <title>PatientVault</title>
</svelte:head>

<BrowserDevBanner />
{@render children()}
<ThemeToggle />
