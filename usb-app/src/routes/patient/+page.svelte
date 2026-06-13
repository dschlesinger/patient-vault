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
    sttStopStream,
    ttsSynthesize,
    ttsStop,
    syncPayloads
  } from '$lib/tauri/commands';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import { MOCK_PATIENT_MESSAGES } from '$lib/mock/data';

  let unlistenLlm: UnlistenFn | null = null;
  let unlistenStt: UnlistenFn | null = null;
  let newPayloadCount = $state(0);

  const PATIENT_SYSTEM_PROMPT = `You are PatientVault AI, a compassionate and helpful assistant for a patient managing their personal health information. You have access to the patient's local vault and can fetch their data, messages from providers, and questionnaires using available tools. Speak clearly and warmly. If the patient wants to go through a questionnaire, walk through it one question at a time.`;

  onMount(async () => {
    chat.clear();

    try {
      newPayloadCount = await syncPayloads();

      await llmStartSession(PATIENT_SYSTEM_PROMPT);

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

      chat.addMessage('assistant', 'Hello! I\'m here to help you manage your health information. What would you like to do today?');
    } catch {
      // Tauri not available — load mock conversation for browser dev
      newPayloadCount = 2;
      for (const m of MOCK_PATIENT_MESSAGES) chat.messages.push(m);
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
      <h1 class="font-black text-lg">Patient session</h1>
    </div>
    {#if newPayloadCount > 0}
      <span class="px-3 py-1 bg-[--color-nb-accent] font-bold nb-border text-sm">
        {newPayloadCount} new message{newPayloadCount > 1 ? 's' : ''}
      </span>
    {/if}
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
