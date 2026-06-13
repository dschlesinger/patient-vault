<script lang="ts">
  import '../app.css';

  let { children } = $props();
</script>

<svelte:head>
  <title>PatientVault</title>
</svelte:head>

{@render children()}
