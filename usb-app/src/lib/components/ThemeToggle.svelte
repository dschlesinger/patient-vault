<script lang="ts">
  import { onMount } from 'svelte';

  let dark = $state(false);

  onMount(() => {
    dark = document.documentElement.classList.contains('dark');
  });

  function toggleTheme() {
    dark = !dark;
    document.documentElement.classList.toggle('dark', dark);
    try {
      localStorage.setItem('theme', dark ? 'dark' : 'light');
    } catch {}
  }
</script>

<button
  type="button"
  class="fixed bottom-4 right-4 z-50 font-bold px-3 py-2 nb-border nb-shadow nb-shadow-hover bg-[--color-nb-surface] text-[--color-nb-black]"
  onclick={toggleTheme}
  aria-label={dark ? 'Switch to light mode' : 'Switch to dark mode'}
>
  {dark ? '☀ Light' : '☾ Dark'}
</button>
