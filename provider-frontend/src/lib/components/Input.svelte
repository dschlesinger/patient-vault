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
    showToggle?: boolean;
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
    showToggle = false,
    oninput
  }: Props = $props();

  let visible = $state(false);
  let resolvedType = $derived(showToggle && visible ? 'text' : type);
</script>

<div class="flex flex-col gap-1">
  {#if label}
    <label for={id} class="font-bold text-sm">{label}</label>
  {/if}
  <div class="relative">
    <input
      {id}
      {name}
      type={resolvedType}
      {placeholder}
      bind:value
      {required}
      {oninput}
      class="w-full px-3 py-2 nb-border bg-[--color-nb-white] font-sans focus:outline-none focus:ring-2 focus:ring-[--color-nb-black] {showToggle ? 'pr-10' : ''} {extraClass}"
    />
    {#if showToggle}
      <button
        type="button"
        onclick={() => (visible = !visible)}
        class="absolute right-0 top-0 h-full px-3 flex items-center text-[--color-nb-text-secondary] hover:text-[--color-nb-black]"
        aria-label={visible ? 'Hide password' : 'Show password'}
      >
        {#if visible}
          <!-- eye-off -->
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94"/>
            <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19"/>
            <line x1="1" y1="1" x2="23" y2="23"/>
          </svg>
        {:else}
          <!-- eye -->
          <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/>
            <circle cx="12" cy="12" r="3"/>
          </svg>
        {/if}
      </button>
    {/if}
  </div>
</div>
