<script lang="ts">
  import { goto } from '$app/navigation';
  import Button from '$lib/components/Button.svelte';
  import ChatBubble from '$lib/components/ChatBubble.svelte';
  import SessionTabBar from '$lib/components/SessionTabBar.svelte';
  import MessagesPanel from '$lib/components/MessagesPanel.svelte';
  import QuestionnairesPanel from '$lib/components/QuestionnairesPanel.svelte';
  import DocumentsPanel from '$lib/components/DocumentsPanel.svelte';
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
    syncPayloads,
    readPayloads,
    listProviderLinks,
    isTauriAvailable,
    type DecryptedPayload,
    type ProviderLink
  } from '$lib/tauri/commands';
  import { listenWhenTauri } from '$lib/tauri/events';
  import { playWavBase64, stopAudio, detectTtsLanguage } from '$lib/audio';
  import { messagePayloads, questionnairePayloads, documentPayloads, type SessionTab } from '$lib/payloads';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import { MOCK_PROVIDER_MESSAGES, MOCK_PAYLOADS, MOCK_PROVIDER_LINKS } from '$lib/mock/data';

  const PROVIDER_SYSTEM_PROMPT = `You are PatientVault AI, assisting a healthcare provider during a patient consultation. You have access to the patient's permitted vault data, questionnaire responses, and meeting transcripts — filtered by the patient's privacy settings. Provide concise, clinically relevant summaries. Do not speculate about excluded data.`;

  let unlistenLlm: UnlistenFn | null = null;
  let unlistenStt: UnlistenFn | null = null;
  let unlistenLlmDone: UnlistenFn | null = null;
  let unlistenTts: UnlistenFn | null = null;
  let payloads = $state<DecryptedPayload[]>([]);
  let providerLinks = $state<ProviderLink[]>([]);
  let syncError = $state<string | null>(null);
  let syncing = $state(false);
  let activeTab = $state<SessionTab>('chat');

  const messages = $derived(messagePayloads(payloads));
  const questionnaires = $derived(questionnairePayloads(payloads));
  const documents = $derived(documentPayloads(payloads));

  async function refreshPayloads() {
    if (!isTauriAvailable()) {
      payloads = MOCK_PAYLOADS;
      providerLinks = MOCK_PROVIDER_LINKS;
      syncError = null;
      return;
    }

    syncError = null;
    syncing = true;
    try {
      await syncPayloads();
      payloads = await readPayloads({});
    } catch (e) {
      syncError = String(e);
    } finally {
      syncing = false;
    }
  }

  onMount(async () => {
    chat.clear();

    if (!isTauriAvailable()) {
      payloads = MOCK_PAYLOADS;
      providerLinks = MOCK_PROVIDER_LINKS;
      for (const m of MOCK_PROVIDER_MESSAGES) chat.messages.push(m);
      return;
    }

    try {
      providerLinks = await listProviderLinks();
      await refreshPayloads();
      await llmStartSession(PROVIDER_SYSTEM_PROMPT, 'provider');

      unlistenLlm = await listenWhenTauri<string>('llm://token', (token) => {
        const lastMsg = chat.messages.at(-1);
        if (lastMsg?.role === 'assistant') {
          lastMsg.content += token;
        } else {
          chat.addMessage('assistant', token);
        }
        chat.setLlmThinking(false);
      });

      unlistenStt = await listenWhenTauri<string>('stt://partial', (transcript) => {
        chat.setStreamingTranscript(transcript);
      });

      unlistenLlmDone = await listenWhenTauri<string>('llm://done', (content) => {
        const text = content.trim();
        if (text) {
          ttsSynthesize(text, detectTtsLanguage(text)).catch(() => {});
        }
      });

      unlistenTts = await listenWhenTauri<string>('tts://audio', (wavBase64) => {
        playWavBase64(wavBase64).catch(() => {});
      });

      chat.addMessage('assistant', 'Good day. I\'m here to help with this patient\'s visit. Patient data is filtered according to their privacy settings. How can I assist?');
    } catch {
      payloads = MOCK_PAYLOADS;
      for (const m of MOCK_PROVIDER_MESSAGES) chat.messages.push(m);
    }
  });

  onDestroy(async () => {
    unlistenLlm?.();
    unlistenStt?.();
    unlistenLlmDone?.();
    unlistenTts?.();
    stopAudio();
    try {
      await ttsStop();
    } catch {}
    try {
      await llmStopSession();
    } catch {}
  });

  async function toggleMic() {
    if (!isTauriAvailable()) return;

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
      // Barge-in: cut off any TTS playback when speaking starts.
      stopAudio();
      try {
        await ttsStop();
      } catch {}
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

<div class="h-screen min-h-0 bg-[--color-nb-white] flex flex-col overflow-hidden">
  <header class="shrink-0 nb-border-thick border-t-0 border-x-0 p-4 flex items-center justify-between bg-[--color-nb-surface]">
    <div class="flex items-center gap-3">
      <Button variant="secondary" onclick={goHome} class="text-sm px-3 py-1">← Home</Button>
      <h1 class="font-black text-lg">Provider session</h1>
    </div>
    <div class="flex items-center gap-2">
      <span class="px-3 py-1 bg-[--color-nb-black] text-[--color-nb-white] font-bold nb-border text-sm">
        🔒 Guardrails active
      </span>
      <Button variant="secondary" class="text-sm px-3 py-1" disabled={syncing} onclick={refreshPayloads}>
        {syncing ? 'Syncing…' : 'Sync'}
      </Button>
    </div>
  </header>

  <SessionTabBar
    {activeTab}
    messageCount={messages.length}
    questionnaireCount={questionnaires.length}
    documentCount={documents.length}
    onchange={(tab) => (activeTab = tab)}
  />

  {#if syncError}
    <div class="shrink-0 mx-4 mt-4 p-3 bg-[--color-nb-danger] text-[--color-nb-black] font-bold nb-border">
      {syncError}
    </div>
  {/if}

  <div class="flex-1 min-h-0 overflow-hidden flex flex-col">
    {#if activeTab === 'chat'}
      <div class="flex-1 min-h-0 overflow-y-auto p-4 flex flex-col">
        {#each chat.messages as message (message.id)}
          <ChatBubble {message} />
        {/each}

        {#if chat.isLlmThinking}
          <div class="flex justify-start mb-3">
            <div class="px-4 py-3 nb-border bg-[--color-nb-surface] nb-shadow">
              <p class="text-[--color-nb-text-secondary] font-medium">Thinking…</p>
            </div>
          </div>
        {/if}

        {#if chat.streamingTranscript}
          <div class="flex justify-end mb-3">
            <div class="max-w-[80%] px-4 py-3 nb-border bg-[--color-nb-muted]">
              <p class="text-sm text-[--color-nb-text-secondary] mb-1">You (speaking…)</p>
              <p class="italic">{chat.streamingTranscript}</p>
            </div>
          </div>
        {/if}
      </div>
    {:else if activeTab === 'messages'}
      <MessagesPanel payloads={messages} {providerLinks} />
    {:else if activeTab === 'questionnaires'}
      <QuestionnairesPanel payloads={questionnaires} {providerLinks} />
    {:else}
      <DocumentsPanel payloads={documents} {providerLinks} />
    {/if}
  </div>

  {#if activeTab === 'chat'}
    <footer class="shrink-0 p-4 nb-border-thick border-b-0 border-x-0 bg-[--color-nb-surface] flex items-center justify-center gap-4">
      <Button
        variant={chat.isMicActive ? 'danger' : 'primary'}
        class="w-32 h-16 text-lg"
        onclick={toggleMic}
        disabled={chat.isLlmThinking || !isTauriAvailable()}
      >
        {chat.isMicActive ? '⏹ Stop' : '🎤 Speak'}
      </Button>
    </footer>
  {/if}
</div>
