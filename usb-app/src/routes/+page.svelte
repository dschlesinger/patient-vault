<script lang="ts">
  import { goto } from '$app/navigation';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import { session } from '$lib/stores/session.svelte';
  import { vaultExists, generateKeypair, isTauriAvailable } from '$lib/tauri/commands';
  import { onMount } from 'svelte';

  let isFirstRun = $state(false);
  let isLoading = $state(true);
  let setupError = $state<string | null>(null);

  onMount(async () => {
    try {
      isFirstRun = !(await vaultExists());
    } catch {
      // Tauri not available in browser dev mode
    } finally {
      isLoading = false;
    }
  });

  async function handleFirstRunSetup() {
    if (!isTauriAvailable()) return;
    setupError = null;
    try {
      await generateKeypair();
      isFirstRun = false;
    } catch (e) {
      setupError = String(e);
    }
  }

  function goPatient() {
    session.set('patient');
    goto('/patient');
  }

  function goProvider() {
    session.set('provider');
    goto('/provider');
  }
</script>

<div class="min-h-screen bg-[--color-nb-white] flex items-center justify-center p-6">
  <div class="w-full max-w-md">
    <div class="mb-10 text-center">
      <h1 class="text-5xl font-black tracking-tight">PatientVault</h1>
      <p class="mt-2 text-lg font-medium text-[--color-nb-text-secondary]">Secure patient-controlled health data</p>
    </div>

    {#if isLoading}
      <p class="text-center font-medium">Loading…</p>

    {:else if isFirstRun}
      <Card>
        <h2 class="text-2xl font-bold mb-3">First run</h2>
        <p class="text-[--color-nb-text-secondary] mb-6">
          PatientVault will generate a unique cryptographic identity for this device and store it on
          the USB drive. This only happens once.
        </p>
        {#if setupError}
          <div class="mb-4 p-3 bg-[--color-nb-danger] text-[--color-nb-black] font-bold nb-border">
            {setupError}
          </div>
        {/if}
        <Button variant="primary" class="w-full" onclick={handleFirstRunSetup} disabled={!isTauriAvailable()}>
          Generate identity and get started
        </Button>
      </Card>

    {:else}
      <div class="flex flex-col gap-4">
        <Button variant="primary" class="w-full py-5 text-xl" onclick={goPatient}>
          I am a patient
        </Button>
        <Button variant="secondary" class="w-full py-5 text-xl" onclick={goProvider}>
          Provider login
        </Button>
      </div>
    {/if}
  </div>
</div>
