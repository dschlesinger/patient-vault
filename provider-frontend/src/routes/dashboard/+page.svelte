<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import type { PageData, ActionData } from './$types';

  let { data, form }: { data: PageData; form: ActionData } = $props();
</script>

<div class="max-w-4xl">
  <div class="flex items-center justify-between mb-8">
    <h2 class="text-3xl font-black">Patients</h2>
    <form method="POST" action="?/generate_code">
      <Button type="submit" variant="primary">+ Pair new patient</Button>
    </form>
  </div>

  {#if form?.pairingCode}
    <Card class="mb-6 border-[--color-nb-accent]">
      <p class="font-bold text-lg mb-1">New pairing code</p>
      <p class="text-5xl font-black tracking-widest text-center py-4">{form.pairingCode}</p>
      <p class="text-sm text-gray-600 text-center">Share this code with the patient in person. Expires in 10 minutes.</p>
    </Card>
  {/if}

  {#if data.patients.length === 0}
    <Card>
      <p class="text-gray-500 font-medium">No patients linked yet. Generate a pairing code to register your first patient.</p>
    </Card>
  {:else}
    <div class="flex flex-col gap-3">
      {#each data.patients as patient}
        <a
          href="/dashboard/{patient.usb_id}"
          class="block bg-[--color-nb-surface] nb-border nb-shadow nb-shadow-hover p-4"
        >
          <div class="flex items-center justify-between">
            <div>
              <p class="font-bold text-lg">{patient.patient_name}</p>
              <p class="text-xs text-gray-500 font-mono mt-0.5">ID: {patient.usb_id.slice(0, 16)}…</p>
            </div>
            <p class="text-sm text-gray-500">Registered {new Date(patient.registered_at).toLocaleDateString()}</p>
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>
