<script lang="ts">
  interface Props {
    class?: string;
    children: import('svelte').Snippet;
  }

  let { class: extraClass = '', children }: Props = $props();
</script>

<div class="bg-[--color-nb-surface] nb-border nb-shadow p-6 {extraClass}">
  {@render children()}
</div>
