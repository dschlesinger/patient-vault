<script lang="ts">
  import { goto } from '$app/navigation';
  import Button from '$lib/components/Button.svelte';
  import ChatBubble from '$lib/components/ChatBubble.svelte';
  import SessionTabBar from '$lib/components/SessionTabBar.svelte';
  import MessagesPanel from '$lib/components/MessagesPanel.svelte';
  import QuestionnairesPanel from '$lib/components/QuestionnairesPanel.svelte';
  import DocumentsPanel from '$lib/components/DocumentsPanel.svelte';
  import ProvidersPanel from '$lib/components/ProvidersPanel.svelte';
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
  import {
    messagePayloads,
    questionnairePayloads,
    documentPayloads,
    PATIENT_SESSION_TABS,
    type SessionTab
  } from '$lib/payloads';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import { MOCK_PATIENT_MESSAGES, MOCK_PAYLOADS, MOCK_PROVIDER_LINKS } from '$lib/mock/data';

  let unlistenLlm: UnlistenFn | null = null;
  let unlistenStt: UnlistenFn | null = null;
  let unlistenLlmDone: UnlistenFn | null = null;
  let unlistenTts: UnlistenFn | null = null;
  let newPayloadCount = $state(0);
  let payloads = $state<DecryptedPayload[]>([]);
  let providerLinks = $state<ProviderLink[]>([]);
  let syncError = $state<string | null>(null);
  let syncing = $state(false);
  let activeTab = $state<SessionTab>('chat');

  const PATIENT_SYSTEM_PROMPT = `You are PatientVault AI, a compassionate and helpful assistant for a patient managing their personal health information. You have access to the patient's local vault and can fetch their data, messages from providers, and questionnaires using available tools. Speak clearly and warmly. If the patient wants to go through a questionnaire, walk through it one question at a time.`;

  const messages = $derived(messagePayloads(payloads));
  const questionnaires = $derived(questionnairePayloads(payloads));
  const documents = $derived(documentPayloads(payloads));

  async function refreshProviderLinks() {
    if (!isTauriAvailable()) {
      providerLinks = MOCK_PROVIDER_LINKS;
      return;
    }
    try {
      providerLinks = await listProviderLinks();
    } catch (e) {
      syncError = String(e);
    }
  }

  async function refreshPayloads() {
    if (!isTauriAvailable()) {
      payloads = MOCK_PAYLOADS;
      newPayloadCount = 0;
      syncError = null;
      return;
    }

    syncError = null;
    syncing = true;
    try {
      const prevDocCount = documentPayloads(payloads).length;
      const prevMessageCount = messagePayloads(payloads).length;
      const prevQuestionnaireCount = questionnairePayloads(payloads).length;
      newPayloadCount = await syncPayloads();
      payloads = await readPayloads({});
      const newDocCount = documentPayloads(payloads).length;
      const newMessageCount = messagePayloads(payloads).length;
      const newQuestionnaireCount = questionnairePayloads(payloads).length;
      if (newDocCount > prevDocCount) {
        activeTab = 'documents';
      } else if (newQuestionnaireCount > prevQuestionnaireCount && activeTab === 'chat') {
        activeTab = 'questionnaires';
      } else if (newMessageCount > prevMessageCount && activeTab === 'chat') {
        activeTab = 'messages';
      }
    } catch (e) {
      syncError = String(e);
    } finally {
      syncing = false;
    }
  }

  function loadBrowserPreview() {
    newPayloadCount = 2;
    payloads = MOCK_PAYLOADS;
    providerLinks = MOCK_PROVIDER_LINKS;
    for (const m of MOCK_PATIENT_MESSAGES) chat.messages.push(m);
  }

  onMount(async () => {
    chat.clear();

    if (!isTauriAvailable()) {
      loadBrowserPreview();
      return;
    }

    try {
      providerLinks = await listProviderLinks();
      await refreshPayloads();

      await llmStartSession(PATIENT_SYSTEM_PROMPT, 'patient');

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

      // When a reply completes, speak it aloud via Piper TTS.
      unlistenLlmDone = await listenWhenTauri<string>('llm://done', (content) => {
        const text = content.trim();
        if (text) {
          ttsSynthesize(text, detectTtsLanguage(text)).catch(() => {});
        }
      });

      unlistenTts = await listenWhenTauri<string>('tts://audio', (wavBase64) => {
        playWavBase64(wavBase64).catch(() => {});
      });

      chat.addMessage('assistant', 'Hello! I\'m here to help you manage your health information. What would you like to do today?');
    } catch (e) {
      syncError = String(e);
      loadBrowserPreview();
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
      // Barge-in: cut off any TTS playback when the patient starts speaking.
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
      <h1 class="font-black text-lg">Patient session</h1>
    </div>
    <div class="flex items-center gap-2">
      {#if newPayloadCount > 0}
        <span class="px-3 py-1 bg-[--color-nb-accent] font-bold nb-border text-sm">
          {newPayloadCount} new
        </span>
      {/if}
      <Button variant="secondary" class="text-sm px-3 py-1" disabled={syncing} onclick={refreshPayloads}>
        {syncing ? 'Syncing…' : 'Sync'}
      </Button>
    </div>
  </header>

  <SessionTabBar
    {activeTab}
    tabs={PATIENT_SESSION_TABS}
    messageCount={messages.length}
    questionnaireCount={questionnaires.length}
    documentCount={documents.length}
    providerCount={providerLinks.length}
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
      <QuestionnairesPanel payloads={questionnaires} {providerLinks} showChatHint />
    {:else if activeTab === 'documents'}
      <DocumentsPanel payloads={documents} {providerLinks} />
    {:else}
      <ProvidersPanel {providerLinks} onRefresh={refreshProviderLinks} />
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
