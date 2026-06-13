<script lang="ts">
  interface Props {
    id?: string;
    name?: string;
    type?: string;
    placeholder?: string;
    value?: string;
    required?: boolean;
    class?: string;
    label?: string;
    oninput?: (e: Event & { currentTarget: HTMLInputElement }) => void;
  }

  let {
    id,
    name,
    type = 'text',
    placeholder,
    value = $bindable(''),
    required = false,
    class: extraClass = '',
    label,
    oninput
  }: Props = $props();
</script>

<div class="flex flex-col gap-1">
  {#if label}
    <label for={id} class="font-bold text-sm">{label}</label>
  {/if}
  <input
    {id}
    {name}
    {type}
    {placeholder}
    bind:value
    {required}
    {oninput}
    class="px-3 py-2 nb-border bg-white font-sans focus:outline-none focus:ring-2 focus:ring-[--color-nb-black] {extraClass}"
  />
</div>
