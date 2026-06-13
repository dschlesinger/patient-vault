<script lang="ts">
  interface Props {
    variant?: 'primary' | 'secondary' | 'danger';
    type?: 'button' | 'submit' | 'reset';
    disabled?: boolean;
    class?: string;
    onclick?: () => void;
    children: import('svelte').Snippet;
  }

  let {
    variant = 'primary',
    type = 'button',
    disabled = false,
    class: extraClass = '',
    onclick,
    children
  }: Props = $props();

  const base =
    'inline-flex items-center justify-center font-bold px-4 py-2 nb-border nb-shadow nb-shadow-hover cursor-pointer select-none disabled:opacity-50 disabled:cursor-not-allowed';

  const variants = {
    primary: 'bg-[--color-nb-accent] text-[--color-nb-black]',
    secondary: 'bg-[--color-nb-white] text-[--color-nb-black]',
    danger: 'bg-[--color-nb-danger] text-white'
  };
</script>

<button {type} {disabled} {onclick} class="{base} {variants[variant]} {extraClass}">
  {@render children()}
</button>
