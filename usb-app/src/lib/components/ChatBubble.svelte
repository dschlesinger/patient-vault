<script lang="ts">
  import type { ChatMessage } from '$lib/stores/chat.svelte';

  interface Props {
    message: ChatMessage;
  }

  let { message }: Props = $props();

  const isAssistant = $derived(message.role === 'assistant');
</script>

<div class="flex {isAssistant ? 'justify-start' : 'justify-end'} mb-3">
  <div
    class="max-w-[80%] px-4 py-3 nb-border {isAssistant
      ? 'bg-[--color-nb-surface] nb-shadow'
      : 'bg-[--color-nb-accent] nb-shadow'}"
  >
    {#if isAssistant}
      <p class="text-xs font-bold text-gray-500 mb-1">PatientVault AI</p>
    {/if}
    <p class="font-sans">{message.content}</p>
    <p class="text-xs text-gray-400 mt-1 text-right">
      {new Date(message.timestamp).toLocaleTimeString()}
    </p>
  </div>
</div>
