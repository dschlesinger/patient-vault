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
      <div class="flex items-center justify-between mb-1">
        <p class="font-bold text-lg">New pairing code</p>
        <div class="flex gap-2">
          <form method="POST" action="?/refresh_code">
            <input type="hidden" name="old_code" value={form.pairingCode} />
            <Button type="submit" variant="secondary">Refresh</Button>
          </form>
          <form method="POST" action="?/cancel_code">
            <input type="hidden" name="old_code" value={form.pairingCode} />
            <Button type="submit" variant="secondary">Cancel</Button>
          </form>
        </div>
      </div>
      <p class="text-5xl font-black tracking-widest text-center py-4">{form.pairingCode}</p>
      <p class="text-sm text-[--color-nb-text-secondary] text-center">Share this code with the patient in person. Expires in 10 minutes.</p>
    </Card>
  {/if}

  {#if data.patients.length === 0}
    <Card>
      <p class="text-[--color-nb-text-secondary] font-medium">No patients linked yet. Generate a pairing code to register your first patient.</p>
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
              <p class="text-xs text-[--color-nb-text-secondary] font-mono mt-0.5">ID: {patient.usb_id.slice(0, 16)}…</p>
            </div>
            <p class="text-sm text-[--color-nb-text-secondary]">Registered {new Date(patient.registered_at).toLocaleDateString()}</p>
          </div>
        </a>
      {/each}
    </div>
  {/if}
</div>
