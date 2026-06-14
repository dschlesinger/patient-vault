<script lang="ts">
	import Card from '$lib/components/Card.svelte';
	import Button from '$lib/components/Button.svelte';
	import { registerPatient, isTauriAvailable, type ProviderLink } from '$lib/tauri/commands';
	import { formatRegisteredAt } from '$lib/payloads';

	interface Props {
		providerLinks: ProviderLink[];
		/** Reload the provider links after a successful connection. */
		onRefresh: () => Promise<void> | void;
	}

	let { providerLinks, onRefresh }: Props = $props();

	let showForm = $state(false);
	let displayName = $state('');
	let yourName = $state('');
	let code = $state('');
	let error = $state<string | null>(null);
	let submitting = $state(false);

	const canSubmit = $derived(
		displayName.trim().length > 0 && yourName.trim().length > 0 && code.trim().length === 6
	);

	function openForm() {
		error = null;
		// Reuse the patient's own name from an existing pairing so they only type it once.
		if (!yourName && providerLinks.length > 0) {
			yourName = providerLinks[0].patient_name;
		}
		showForm = true;
	}

	function cancelForm() {
		showForm = false;
		displayName = '';
		code = '';
		error = null;
	}

	async function connect() {
		error = null;
		if (!isTauriAvailable()) {
			error = 'Connecting a provider requires the desktop app (`pnpm dev:usb`).';
			return;
		}
		submitting = true;
		try {
			await registerPatient(yourName.trim(), code.trim(), displayName.trim());
			await onRefresh();
			displayName = '';
			code = '';
			showForm = false;
		} catch (e) {
			error = String(e);
		} finally {
			submitting = false;
		}
	}
</script>

<div class="flex-1 min-h-0 overflow-y-auto p-4">
	<div class="flex flex-col gap-4">
		<div class="flex items-center justify-between gap-3">
			<h2 class="font-black text-lg">Connected providers</h2>
			{#if !showForm}
				<Button variant="primary" class="text-sm px-3 py-1" onclick={openForm}>
					+ Connect a provider
				</Button>
			{/if}
		</div>

		{#if showForm}
			<Card class="!p-4">
				<h3 class="font-bold text-base mb-1">Connect a provider</h3>
				<p class="text-sm text-[--color-nb-text-secondary] mb-4">
					Enter the 6-digit code shown in the provider portal during your visit, and give the
					provider a name you'll recognize.
				</p>

				{#if error}
					<div class="mb-4 p-3 bg-[--color-nb-danger] text-[--color-nb-black] font-bold nb-border">
						{error}
					</div>
				{/if}

				<div class="flex flex-col gap-3 mb-4">
					<label class="flex flex-col gap-1">
						<span class="text-sm font-bold">Provider name</span>
						<input
							type="text"
							bind:value={displayName}
							placeholder="e.g. Dr. Patel (Cardiology)"
							class="px-3 py-2 nb-border bg-[--color-nb-white] font-sans"
						/>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-sm font-bold">Your name</span>
						<input
							type="text"
							bind:value={yourName}
							placeholder="Your name"
							class="px-3 py-2 nb-border bg-[--color-nb-white] font-sans"
						/>
					</label>
					<label class="flex flex-col gap-1">
						<span class="text-sm font-bold">Pairing code</span>
						<input
							type="text"
							inputmode="numeric"
							maxlength="6"
							bind:value={code}
							placeholder="6-digit code"
							class="px-3 py-2 nb-border bg-[--color-nb-white] font-sans tracking-widest text-center text-xl font-black"
						/>
					</label>
				</div>

				<div class="flex gap-2">
					<Button
						variant="primary"
						class="flex-1"
						disabled={submitting || !canSubmit}
						onclick={connect}
					>
						{submitting ? 'Connecting…' : 'Connect'}
					</Button>
					<Button variant="secondary" class="px-4" disabled={submitting} onclick={cancelForm}>
						Cancel
					</Button>
				</div>
			</Card>
		{/if}

		{#if providerLinks.length === 0}
			{#if !showForm}
				<Card class="!p-6 text-center">
					<p class="font-bold text-lg mb-1">No providers connected yet</p>
					<p class="text-sm text-[--color-nb-text-secondary]">
						Connect a provider to start receiving their messages, questionnaires, and documents.
					</p>
				</Card>
			{/if}
		{:else}
			<ul class="flex flex-col gap-3">
				{#each providerLinks as link (link.provider_id || link.registered_at)}
					<li>
						<Card class="!p-4">
							<p class="font-bold">{link.display_name || 'Your provider'}</p>
							<p class="text-xs text-[--color-nb-text-secondary] mt-0.5">
								Registered as {link.patient_name}
							</p>
							<p class="text-xs text-[--color-nb-text-tertiary] mt-0.5">
								Paired {formatRegisteredAt(link.registered_at)}
							</p>
						</Card>
					</li>
				{/each}
			</ul>
		{/if}
	</div>
</div>
