<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import Input from '$lib/components/Input.svelte';
  import type { PageData, ActionData } from './$types';

  let { data, form }: { data: PageData; form: ActionData } = $props();

  let messageContent = $state('');
  let activeTab = $state<'messages' | 'questionnaire' | 'document'>('messages');
</script>

<div class="max-w-4xl">
  <div class="mb-6">
    <a href="/dashboard" class="text-sm font-bold hover:underline">← Back to patients</a>
    <h2 class="text-3xl font-black mt-2">{data.patient.patient_name}</h2>
    <p class="text-xs font-mono text-gray-500">USB ID: {data.patient.usb_id}</p>
  </div>

  <!-- Tab nav -->
  <div class="flex gap-2 mb-6">
    {#each [['messages', 'Sent history'], ['questionnaire', 'Send questionnaire'], ['document', 'Send document']] as [tab, label]}
      <button
        class="px-4 py-2 font-bold nb-border {activeTab === tab ? 'bg-[--color-nb-accent] nb-shadow' : 'bg-white'}"
        onclick={() => (activeTab = tab as typeof activeTab)}
      >
        {label}
      </button>
    {/each}
  </div>

  {#if activeTab === 'messages'}
    <div class="flex flex-col gap-3">
      {#if data.sentLog.length === 0}
        <Card><p class="text-gray-500">No messages sent yet.</p></Card>
      {:else}
        {#each data.sentLog as entry}
          <div class="bg-[--color-nb-surface] nb-border p-4 flex items-center justify-between">
            <div>
              <span class="inline-block px-2 py-0.5 text-xs font-bold nb-border bg-white mr-2">{entry.type}</span>
              <span class="text-sm text-gray-600">{new Date(entry.sent_at).toLocaleString()}</span>
            </div>
          </div>
        {/each}
      {/if}
    </div>

  {:else if activeTab === 'questionnaire'}
    {#if form?.error}
      <div class="mb-4 p-3 bg-[--color-nb-danger] text-white font-bold nb-border">{form.error}</div>
    {/if}
    {#if form?.success}
      <div class="mb-4 p-3 bg-[--color-nb-success] text-white font-bold nb-border">Questionnaire sent!</div>
    {/if}
    <Card>
      <h3 class="font-bold text-xl mb-4">Send questionnaire</h3>
      <form method="POST" action="?/send_questionnaire" class="flex flex-col gap-4">
        <textarea
          name="content"
          placeholder="Enter questions (one per line)…"
          rows={8}
          class="w-full px-3 py-2 nb-border bg-white font-sans focus:outline-none focus:ring-2 focus:ring-[--color-nb-black] resize-y"
          required
        ></textarea>
        <Button type="submit" variant="primary">Send encrypted questionnaire</Button>
      </form>
    </Card>

  {:else if activeTab === 'document'}
    {#if form?.error}
      <div class="mb-4 p-3 bg-[--color-nb-danger] text-white font-bold nb-border">{form.error}</div>
    {/if}
    {#if form?.success}
      <div class="mb-4 p-3 bg-[--color-nb-success] text-white font-bold nb-border">Document uploaded!</div>
    {/if}
    <Card>
      <h3 class="font-bold text-xl mb-4">Send document</h3>
      <p class="text-sm text-gray-600 mb-4">File is encrypted in your browser before upload. Max 50MB.</p>
      <form method="POST" action="?/send_document" enctype="multipart/form-data" class="flex flex-col gap-4">
        <input
          type="file"
          name="file"
          required
          class="px-3 py-2 nb-border bg-white w-full"
        />
        <Button type="submit" variant="primary">Encrypt and upload</Button>
      </form>
    </Card>
  {/if}
</div>
