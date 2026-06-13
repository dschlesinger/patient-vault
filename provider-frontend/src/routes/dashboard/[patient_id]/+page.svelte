<script lang="ts">
  import Button from '$lib/components/Button.svelte';
  import Card from '$lib/components/Card.svelte';
  import type { PageData, ActionData } from './$types';

  let { data, form }: { data: PageData; form: ActionData } = $props();

  type Tab = 'history' | 'message' | 'questionnaire' | 'document';
  let activeTab = $state<Tab>('history');

  // Message compose
  let messageText = $state('');

  // Questionnaire compose — list of question strings
  let questions = $state<string[]>(['']);

  function addQuestion() {
    questions.push('');
  }

  function removeQuestion(i: number) {
    questions.splice(i, 1);
    if (questions.length === 0) questions.push('');
  }

  const TABS: [Tab, string][] = [
    ['history', 'Sent history'],
    ['message', 'Send message'],
    ['questionnaire', 'Send questionnaire'],
    ['document', 'Send document']
  ];
</script>

<div class="max-w-4xl">
  <div class="mb-6">
    <a href="/dashboard" class="text-sm font-bold hover:underline">← Back to patients</a>
    <h2 class="text-3xl font-black mt-2">{data.patient.patient_name}</h2>
    <p class="text-xs font-mono text-gray-500">USB ID: {data.patient.usb_id}</p>
  </div>

  <!-- Tab nav -->
  <div class="flex gap-2 mb-6 flex-wrap">
    {#each TABS as [tab, label]}
      <button
        class="px-4 py-2 font-bold nb-border {activeTab === tab
          ? 'bg-[--color-nb-accent] nb-shadow'
          : 'bg-white hover:bg-[--color-nb-surface]'}"
        onclick={() => (activeTab = tab)}
      >
        {label}
      </button>
    {/each}
  </div>

  <!-- ── Sent history ──────────────────────────────────────── -->
  {#if activeTab === 'history'}
    <div class="flex flex-col gap-3">
      {#if data.sentLog.length === 0}
        <Card><p class="text-gray-500">Nothing sent to this patient yet.</p></Card>
      {:else}
        {#each data.sentLog as entry}
          <div class="bg-[--color-nb-surface] nb-border p-4 flex items-center justify-between">
            <div class="flex items-center gap-3">
              <span class="inline-block px-2 py-0.5 text-xs font-bold nb-border bg-white uppercase tracking-wide">
                {entry.type}
              </span>
              <span class="text-sm text-gray-600">{new Date(entry.sent_at).toLocaleString()}</span>
            </div>
            <span class="text-xs font-mono text-gray-400">{entry.payload_ref_id.slice(0, 8)}…</span>
          </div>
        {/each}
      {/if}
    </div>

  <!-- ── Send message ──────────────────────────────────────── -->
  {:else if activeTab === 'message'}
    {#if form?.success}
      <div class="mb-4 p-3 bg-[--color-nb-success] text-white font-bold nb-border">Message sent!</div>
    {/if}
    {#if form?.error}
      <div class="mb-4 p-3 bg-[--color-nb-danger] text-white font-bold nb-border">{form.error}</div>
    {/if}
    <Card>
      <h3 class="font-bold text-xl mb-1">Send message</h3>
      <p class="text-sm text-gray-600 mb-4">
        Message is encrypted in your browser before leaving this page. Only the patient can read it.
      </p>
      <form method="POST" action="?/send_message" class="flex flex-col gap-4">
        <textarea
          name="content"
          bind:value={messageText}
          placeholder="Type your message to the patient…"
          rows={6}
          required
          class="w-full px-3 py-2 nb-border bg-white font-sans focus:outline-none focus:ring-2 focus:ring-[--color-nb-black] resize-y"
        ></textarea>
        <div class="flex justify-end">
          <Button type="submit" variant="primary" disabled={!messageText.trim()}>
            Send encrypted message
          </Button>
        </div>
      </form>
    </Card>

  <!-- ── Send questionnaire ────────────────────────────────── -->
  {:else if activeTab === 'questionnaire'}
    {#if form?.success}
      <div class="mb-4 p-3 bg-[--color-nb-success] text-white font-bold nb-border">Questionnaire sent!</div>
    {/if}
    {#if form?.error}
      <div class="mb-4 p-3 bg-[--color-nb-danger] text-white font-bold nb-border">{form.error}</div>
    {/if}
    <Card>
      <h3 class="font-bold text-xl mb-1">Send questionnaire</h3>
      <p class="text-sm text-gray-600 mb-6">
        Questions are encrypted before sending. The patient's AI will walk through them one by one.
      </p>
      <form method="POST" action="?/send_questionnaire" class="flex flex-col gap-6">
        <div class="flex flex-col gap-3">
          {#each questions as _, i}
            <div class="flex gap-2 items-start">
              <div class="flex items-center justify-center w-7 h-10 shrink-0 font-black text-gray-400 text-sm">
                {i + 1}.
              </div>
              <input
                type="text"
                name="question"
                bind:value={questions[i]}
                placeholder="Enter question…"
                required
                class="flex-1 px-3 py-2 nb-border bg-white font-sans focus:outline-none focus:ring-2 focus:ring-[--color-nb-black]"
              />
              <button
                type="button"
                onclick={() => removeQuestion(i)}
                class="px-3 py-2 nb-border bg-white font-bold text-[--color-nb-danger] hover:bg-red-50 shrink-0"
                title="Remove question"
              >
                ✕
              </button>
            </div>
          {/each}
        </div>

        <button
          type="button"
          onclick={addQuestion}
          class="flex items-center gap-2 px-4 py-2 nb-border bg-[--color-nb-surface] font-bold hover:bg-[--color-nb-muted] w-fit"
        >
          <span class="text-lg leading-none">+</span> Add question
        </button>

        <div class="flex justify-end pt-2 border-t-2 border-[--color-nb-black]">
          <Button
            type="submit"
            variant="primary"
            disabled={questions.every((q) => !q.trim())}
          >
            Send encrypted questionnaire
          </Button>
        </div>
      </form>
    </Card>

  <!-- ── Send document ─────────────────────────────────────── -->
  {:else if activeTab === 'document'}
    {#if form?.success}
      <div class="mb-4 p-3 bg-[--color-nb-success] text-white font-bold nb-border">Document uploaded!</div>
    {/if}
    {#if form?.error}
      <div class="mb-4 p-3 bg-[--color-nb-danger] text-white font-bold nb-border">{form.error}</div>
    {/if}
    <Card>
      <h3 class="font-bold text-xl mb-1">Send document</h3>
      <p class="text-sm text-gray-600 mb-4">
        File is encrypted in your browser before upload. Max 50 MB. Only the patient can decrypt it.
      </p>
      <form method="POST" action="?/send_document" enctype="multipart/form-data" class="flex flex-col gap-4">
        <input
          type="file"
          name="file"
          required
          class="px-3 py-2 nb-border bg-white w-full cursor-pointer file:mr-3 file:px-3 file:py-1 file:nb-border file:bg-[--color-nb-accent] file:font-bold file:cursor-pointer"
        />
        <div class="flex justify-end">
          <Button type="submit" variant="primary">Encrypt and upload</Button>
        </div>
      </form>
    </Card>
  {/if}
</div>
