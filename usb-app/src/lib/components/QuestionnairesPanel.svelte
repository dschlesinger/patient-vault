<script lang="ts">
	import Card from '$lib/components/Card.svelte';
	import type { DecryptedPayload, ProviderLink } from '$lib/tauri/commands';
	import { parseQuestionnaireQuestions, providerLabel } from '$lib/payloads';

	interface Props {
		payloads: DecryptedPayload[];
		providerLinks?: ProviderLink[];
		/** Show voice-assistant hint (patient session). */
		showChatHint?: boolean;
	}

	let { payloads, providerLinks = [], showChatHint = false }: Props = $props();
</script>

<div class="flex-1 min-h-0 overflow-y-auto p-4">
	{#if payloads.length === 0}
		<Card class="!p-6 text-center">
			<p class="font-bold text-lg mb-1">No questionnaires yet</p>
			<p class="text-sm text-[--color-nb-text-secondary]">
				Pre-visit questionnaires from your providers will appear here after you sync.
			</p>
		</Card>
	{:else}
		<div class="flex flex-col gap-4">
			{#each payloads as payload (payload.id)}
				{@const questions = parseQuestionnaireQuestions(payload.content)}
				<Card class="!p-0 overflow-hidden">
					<div class="px-4 py-3 bg-[--color-nb-muted] nb-border border-t-0 border-x-0 flex items-center justify-between gap-3 flex-wrap">
						<div class="flex items-center gap-2 flex-wrap">
							<span class="font-black text-base">Questionnaire</span>
							<span
								class="inline-block px-2 py-0.5 text-xs font-bold nb-border bg-[--color-nb-white] uppercase tracking-wide"
							>
								{questions.length} question{questions.length === 1 ? '' : 's'}
							</span>
							<span class="text-xs text-[--color-nb-text-secondary]">
								{providerLabel(payload.provider_id, providerLinks)}
							</span>
						</div>
						<span class="text-xs text-[--color-nb-text-tertiary] shrink-0">
							{new Date(payload.received_at).toLocaleString()}
						</span>
					</div>

					<ol class="p-4 flex flex-col gap-3 list-none m-0">
						{#each questions as question, i (i)}
							<li class="flex gap-3 items-start">
								<span
									class="shrink-0 w-7 h-7 flex items-center justify-center font-black text-sm nb-border bg-[--color-nb-accent]"
								>
									{i + 1}
								</span>
								<p class="text-sm pt-1 flex-1">{question}</p>
							</li>
						{/each}
					</ol>

					{#if showChatHint}
						<p class="px-4 pb-4 text-xs text-[--color-nb-text-secondary]">
							Open <span class="font-bold">Chat</span> and ask to go through this questionnaire — the
							assistant will walk you through each question by voice.
						</p>
					{/if}
				</Card>
			{/each}
		</div>
	{/if}
</div>
