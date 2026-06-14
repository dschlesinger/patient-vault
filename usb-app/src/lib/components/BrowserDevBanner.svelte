<script lang="ts">
  import { isTauriAvailable } from '$lib/tauri/env';

  const showBanner = !isTauriAvailable();
</script>

{#if showBanner}
  <div class="shrink-0 px-4 py-2 bg-[--color-nb-accent] nb-border-thick border-t-0 border-x-0 text-sm font-bold text-center">
    Browser preview — run <code class="font-mono">pnpm dev:usb</code> for the desktop app. Tauri features use mock data here.
  </div>
{/if}
