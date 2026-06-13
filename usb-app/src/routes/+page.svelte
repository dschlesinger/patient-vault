<script lang="ts">
  import { goto } from '$app/navigation';
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import { session } from '$lib/stores/session.svelte';
  import {
    vaultExists,
    generateKeypair,
    getUsbId,
    registerPatient,
    listProviderLinks,
    isTauriAvailable,
    type ProviderLink
  } from '$lib/tauri/commands';
  import { onMount } from 'svelte';
  import { formatRegisteredAt } from '$lib/payloads';

  let isFirstRun = $state(false);
  let isLoading = $state(true);
  let setupComplete = $state(false);
  let providers = $state<ProviderLink[]>([]);
  let usbId = $state('');
  let patientName = $state('');
  let providerCode = $state('');
  let registerError = $state<string | null>(null);
  let registering = $state(false);
  let showPairForm = $state(false);

  const hasProviders = $derived(providers.length > 0);

  onMount(async () => {
    try {
      const exists = await vaultExists();
      isFirstRun = !exists;
      if (exists) {
        providers = await listProviderLinks();
        usbId = await getUsbId();
      }
    } catch {
      // Tauri not available in browser dev mode
    } finally {
      isLoading = false;
    }
  });

  async function handleFirstRunSetup() {
    if (!isTauriAvailable()) return;
    try {
      await generateKeypair();
      setupComplete = true;
      isFirstRun = false;
      usbId = await getUsbId();
      showPairForm = true;
    } catch (e) {
      registerError = String(e);
    }
  }

  async function handleRegister() {
    if (!isTauriAvailable()) {
      registerError = 'Registration requires the desktop app (`pnpm dev:usb`).';
      return;
    }
    registerError = null;
    registering = true;
    try {
      await registerPatient(patientName, providerCode);
      providers = await listProviderLinks();
      providerCode = '';
      showPairForm = false;
    } catch (e) {
      registerError = String(e);
    } finally {
      registering = false;
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
          PatientVault will generate a unique cryptographic identity for this device and store it in
          Persistent Storage. This only happens once.
        </p>
        <Button variant="primary" class="w-full" onclick={handleFirstRunSetup} disabled={!isTauriAvailable()}>
          Generate identity and get started
        </Button>
      </Card>

    {:else if showPairForm || (!hasProviders && usbId)}
      <Card>
        {#if setupComplete}
          <p class="font-bold text-[--color-nb-success] mb-4">✓ Identity created</p>
        {/if}

        {#if hasProviders}
          <Button variant="secondary" class="mb-4 text-sm px-3 py-1" onclick={() => (showPairForm = false)}>
            ← Back
          </Button>
        {/if}

        <p class="text-sm text-[--color-nb-text-secondary] mb-2">Your device ID:</p>
        <p class="font-mono text-xs break-all nb-border bg-[--color-nb-surface] p-3 mb-6">{usbId}</p>

        <h2 class="text-xl font-bold mb-2">
          {hasProviders ? 'Pair with another provider' : 'Register with provider'}
        </h2>
        <p class="text-[--color-nb-text-secondary] mb-4">
          Enter the 6-digit code shown on the provider portal during your visit.
        </p>

        {#if registerError}
          <div class="mb-4 p-3 bg-[--color-nb-danger] text-[--color-nb-black] font-bold nb-border">
            {registerError}
          </div>
        {/if}

        <div class="flex flex-col gap-3 mb-4">
          <input
            type="text"
            bind:value={patientName}
            placeholder="Your name"
            class="px-3 py-2 nb-border bg-[--color-nb-white] font-sans"
          />
          <input
            type="text"
            inputmode="numeric"
            maxlength="6"
            bind:value={providerCode}
            placeholder="6-digit pairing code"
            class="px-3 py-2 nb-border bg-[--color-nb-white] font-sans tracking-widest text-center text-xl font-black"
          />
        </div>

        <Button
          variant="primary"
          class="w-full"
          disabled={registering || !patientName.trim() || providerCode.trim().length !== 6}
          onclick={handleRegister}
        >
          {registering ? 'Registering…' : 'Register with provider'}
        </Button>
      </Card>

    {:else}
      <div class="flex flex-col gap-4">
        <Card>
          <p class="font-bold text-[--color-nb-success] mb-3">
            ✓ Paired with {providers.length} provider{providers.length === 1 ? '' : 's'}
          </p>
          <ul class="flex flex-col gap-2">
            {#each providers as link (link.provider_id || link.registered_at)}
              <li class="nb-border bg-[--color-nb-surface] p-3">
                <p class="font-bold">
                  {#if link.provider_id}
                    Provider {link.provider_id.slice(0, 8)}…
                  {:else}
                    Provider (legacy link)
                  {/if}
                </p>
                <p class="text-xs text-[--color-nb-text-secondary] mt-0.5">{link.patient_name}</p>
                <p class="text-xs text-[--color-nb-text-tertiary] mt-0.5">
                  Paired {formatRegisteredAt(link.registered_at)}
                </p>
              </li>
            {/each}
          </ul>
          <p class="text-xs font-mono text-[--color-nb-text-tertiary] mt-3 break-all">Device: {usbId.slice(0, 16)}…</p>
        </Card>

        <Button variant="secondary" class="w-full" onclick={() => (showPairForm = true)}>
          + Pair with another provider
        </Button>

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
