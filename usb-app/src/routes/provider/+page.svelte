<script lang="ts">
  import { goto } from '$app/navigation';
  import { listen } from '@tauri-apps/api/event';
  import Button from '$lib/components/Button.svelte';
  import ChatBubble from '$lib/components/ChatBubble.svelte';
  import { session } from '$lib/stores/session.svelte';
  import { chat } from '$lib/stores/chat.svelte';
  import {
    llmStartSession,
    llmSendMessage,
    llmStopSession,
    sttStartStream,
    sttStopStream
  } from '$lib/tauri/commands';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  // Provider system prompt: guardrails are injected by the Rust backend.
  // The patient's guardrail text + excluded vault entries are applied server-side
  // before this session begins, so the LLM never sees private data.
  const PROVIDER_SYSTEM_PROMPT = `You are PatientVault AI, assisting a healthcare provider during a patient consultation. You have access to the patient's permitted vault data, questionnaire responses, and meeting transcripts — filtered by the patient's privacy settings. Provide concise, clinically relevant summaries. Do not speculate about excluded data.`;

  let unlistenLlm: UnlistenFn | null = null;
  let unlistenStt: UnlistenFn | null = null;

  onMount(async () => {
    chat.clear();

    try {
      await llmStartSession(PROVIDER_SYSTEM_PROMPT);

      unlistenLlm = await listen<string>('llm://token', (event) => {
        const lastMsg = chat.messages.at(-1);
        if (lastMsg?.role === 'assistant') {
          lastMsg.content += event.payload;
        } else {
          chat.addMessage('assistant', event.payload);
        }
        chat.setLlmThinking(false);
      });

      unlistenStt = await listen<string>('stt://partial', (event) => {
        chat.setStreamingTranscript(event.payload);
      });

      chat.addMessage('assistant', 'Good day. I\'m here to help with this patient\'s visit. Patient data is filtered according to their privacy settings. How can I assist?');
    } catch {
      // Tauri not available in browser dev
    }
  });

  onDestroy(async () => {
    unlistenLlm?.();
    unlistenStt?.();
    try {
      await llmStopSession();
    } catch {}
  });

  async function toggleMic() {
    if (chat.isMicActive) {
      chat.setMicActive(false);
      try {
        const transcript = await sttStopStream();
        chat.setStreamingTranscript('');
        if (transcript.trim()) {
          chat.addMessage('user', transcript);
          chat.setLlmThinking(true);
          await llmSendMessage(transcript);
        }
      } catch {}
    } else {
      chat.setMicActive(true);
      try {
        await sttStartStream();
      } catch {}
    }
  }

  function goHome() {
    session.set('none');
    goto('/');
  }
</script>

<div class="min-h-screen bg-[--color-nb-white] flex flex-col">
  <!-- Header -->
  <header class="nb-border-thick border-t-0 border-x-0 p-4 flex items-center justify-between bg-[--color-nb-surface]">
    <div class="flex items-center gap-3">
      <Button variant="secondary" onclick={goHome} class="text-sm px-3 py-1">← Home</Button>
      <h1 class="font-black text-lg">Provider session</h1>
    </div>
    <!-- Guardrail active indicator — always visible in provider sessions -->
    <span class="px-3 py-1 bg-[--color-nb-black] text-white font-bold nb-border text-sm">
      🔒 Guardrails active
    </span>
  </header>

  <!-- Chat messages -->
  <div class="flex-1 overflow-y-auto p-4 flex flex-col">
    {#each chat.messages as message (message.id)}
      <ChatBubble {message} />
    {/each}

    {#if chat.isLlmThinking}
      <div class="flex justify-start mb-3">
        <div class="px-4 py-3 nb-border bg-[--color-nb-surface] nb-shadow">
          <p class="text-gray-500 font-medium">Thinking…</p>
        </div>
      </div>
    {/if}

    {#if chat.streamingTranscript}
      <div class="flex justify-end mb-3">
        <div class="max-w-[80%] px-4 py-3 nb-border bg-[--color-nb-muted]">
          <p class="text-sm text-gray-500 mb-1">You (speaking…)</p>
          <p class="italic">{chat.streamingTranscript}</p>
        </div>
      </div>
    {/if}
  </div>

  <!-- Mic input area -->
  <footer class="p-4 nb-border-thick border-b-0 border-x-0 bg-[--color-nb-surface] flex items-center justify-center gap-4">
    <Button
      variant={chat.isMicActive ? 'danger' : 'primary'}
      class="w-32 h-16 text-lg"
      onclick={toggleMic}
      disabled={chat.isLlmThinking}
    >
      {chat.isMicActive ? '⏹ Stop' : '🎤 Speak'}
    </Button>
  </footer>
</div>
