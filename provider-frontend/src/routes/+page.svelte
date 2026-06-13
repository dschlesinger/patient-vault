<script lang="ts">
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  let { form } = $props();

  let email = $state('');
  let password = $state('');
</script>

<div class="min-h-screen bg-[--color-nb-white] flex items-center justify-center p-6">
  <div class="w-full max-w-md">
    <div class="mb-8">
      <h1 class="text-4xl font-black tracking-tight">PatientVault</h1>
      <p class="mt-1 text-lg font-medium text-gray-600">Provider Portal</p>
    </div>

    <Card>
      <h2 class="text-2xl font-bold mb-6">Sign in</h2>

      {#if form?.error}
        <div class="mb-4 p-3 bg-[--color-nb-danger] text-white font-bold nb-border">
          {form.error}
        </div>
      {/if}

      <form method="POST" class="flex flex-col gap-4">
        <Input
          id="email"
          name="email"
          type="email"
          label="Email"
          placeholder="you@clinic.com"
          bind:value={email}
          required
        />
        <Input
          id="password"
          name="password"
          type="password"
          label="Password"
          placeholder="••••••••"
          bind:value={password}
          required
        />
        <Button type="submit" variant="primary" class="w-full mt-2">Sign in</Button>
      </form>
    </Card>
  </div>
</div>
