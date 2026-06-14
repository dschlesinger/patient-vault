<script lang="ts">
	import { open } from '@tauri-apps/plugin-shell';
	import Card from '$lib/components/Card.svelte';
	import Button from '$lib/components/Button.svelte';
	import type { DecryptedPayload, ProviderLink } from '$lib/tauri/commands';
	import { isTauriAvailable } from '$lib/tauri/env';
	import {
		parseDocumentMeta,
		formatFileSize,
		fileTypeLabel,
		providerLabel
	} from '$lib/payloads';

	interface Props {
		payloads: DecryptedPayload[];
		providerLinks?: ProviderLink[];
	}

	let { payloads, providerLinks = [] }: Props = $props();
	let openError = $state<string | null>(null);

	async function openDocument(path: string) {
		openError = null;
		if (!isTauriAvailable()) {
			openError = 'Opening files requires the desktop app.';
			return;
		}
		try {
			await open(path);
		} catch (e) {
			openError = String(e);
		}
	}
</script>

<div class="flex-1 min-h-0 overflow-y-auto p-4">
	{#if openError}
		<div class="mb-4 p-3 bg-[--color-nb-danger] text-[--color-nb-black] font-bold nb-border">
			{openError}
		</div>
	{/if}

	{#if payloads.length === 0}
		<Card class="!p-6 text-center">
			<p class="font-bold text-lg mb-1">No documents yet</p>
			<p class="text-sm text-[--color-nb-text-secondary]">
				Encrypted documents from your providers will appear here after you sync.
			</p>
		</Card>
	{:else}
		<div class="flex flex-col gap-3">
			{#each payloads as payload (payload.id)}
				{@const doc = parseDocumentMeta(payload.content)}
				<Card class="!p-4">
					<div class="flex items-start justify-between gap-3">
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2 mb-1 flex-wrap">
								<span
									class="inline-block px-2 py-0.5 text-xs font-bold nb-border bg-[--color-nb-white] uppercase tracking-wide shrink-0"
								>
									{fileTypeLabel(doc?.type)}
								</span>
								<p class="font-bold truncate">{doc?.name ?? 'Document'}</p>
							</div>
							<p class="text-xs text-[--color-nb-text-secondary]">
								{providerLabel(payload.provider_id || doc?.provider_id || '', providerLinks)}
							</p>
							{#if doc?.size}
								<p class="text-xs text-[--color-nb-text-tertiary] mt-1">{formatFileSize(doc.size)}</p>
							{/if}
							<p class="text-xs text-[--color-nb-text-tertiary] mt-2">
								Received {new Date(payload.received_at).toLocaleString()}
							</p>
						</div>
						{#if doc?.path}
							<Button
								variant="secondary"
								class="text-sm px-3 py-1 shrink-0"
								onclick={() => openDocument(doc.path)}
							>
								Open
							</Button>
						{/if}
					</div>
				</Card>
			{/each}
		</div>
	{/if}
</div>
