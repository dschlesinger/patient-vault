<script lang="ts">
	import Card from '$lib/components/Card.svelte';
	import type { DecryptedPayload, ProviderLink } from '$lib/tauri/commands';
	import { parseMessageContent, providerLabel } from '$lib/payloads';

	interface Props {
		payloads: DecryptedPayload[];
		providerLinks?: ProviderLink[];
	}

	let { payloads, providerLinks = [] }: Props = $props();
</script>

<div class="flex-1 min-h-0 overflow-y-auto p-4">
	{#if payloads.length === 0}
		<Card class="!p-6 text-center">
			<p class="font-bold text-lg mb-1">No messages yet</p>
			<p class="text-sm text-[--color-nb-text-secondary]">
				Messages from your providers will appear here after you sync.
			</p>
		</Card>
	{:else}
		<div class="flex flex-col gap-3">
			{#each payloads as payload (payload.id)}
				<Card class="!p-4">
					<div class="flex items-center justify-between gap-3 mb-2 flex-wrap">
						<div class="flex items-center gap-2">
							<span
								class="inline-block px-2 py-0.5 text-xs font-bold nb-border bg-[--color-nb-white] uppercase tracking-wide"
							>
								Message
							</span>
							<span class="text-xs text-[--color-nb-text-secondary]">
								{providerLabel(payload.provider_id, providerLinks)}
							</span>
						</div>
						<span class="text-xs text-[--color-nb-text-tertiary] shrink-0">
							{new Date(payload.received_at).toLocaleString()}
						</span>
					</div>
					<p class="text-sm whitespace-pre-wrap">{parseMessageContent(payload.content)}</p>
				</Card>
			{/each}
		</div>
	{/if}
</div>
