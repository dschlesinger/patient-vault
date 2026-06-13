<script lang="ts">
	import type { SessionTab } from '$lib/payloads';
	import { SESSION_TABS } from '$lib/payloads';

	interface Props {
		activeTab: SessionTab;
		messageCount?: number;
		questionnaireCount?: number;
		documentCount?: number;
		onchange: (tab: SessionTab) => void;
	}

	let {
		activeTab,
		messageCount = 0,
		questionnaireCount = 0,
		documentCount = 0,
		onchange
	}: Props = $props();
</script>

<nav class="shrink-0 flex gap-2 px-4 py-3 nb-border-thick border-t-0 border-x-0 bg-[--color-nb-white] overflow-x-auto">
	{#each SESSION_TABS as [tab, label]}
		<button
			type="button"
			class="px-4 py-2 font-bold nb-border flex items-center gap-2 shrink-0 {activeTab === tab
				? 'bg-[--color-nb-accent] nb-shadow'
				: 'bg-[--color-nb-white] hover:bg-[--color-nb-surface]'}"
			onclick={() => onchange(tab)}
		>
			{label}
			{#if tab === 'messages' && messageCount > 0}
				<span class="text-xs px-1.5 py-0.5 nb-border bg-[--color-nb-white]">{messageCount}</span>
			{/if}
			{#if tab === 'questionnaires' && questionnaireCount > 0}
				<span class="text-xs px-1.5 py-0.5 nb-border bg-[--color-nb-white]">{questionnaireCount}</span>
			{/if}
			{#if tab === 'documents' && documentCount > 0}
				<span class="text-xs px-1.5 py-0.5 nb-border bg-[--color-nb-white]">{documentCount}</span>
			{/if}
		</button>
	{/each}
</nav>
