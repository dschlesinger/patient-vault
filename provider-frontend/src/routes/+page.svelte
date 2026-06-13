<script lang="ts">
  import Card from '$lib/components/Card.svelte';
  import Button from '$lib/components/Button.svelte';
  import Input from '$lib/components/Input.svelte';

  let { form } = $props();

  type Mode = 'signin' | 'signup';
  let mode = $state<Mode>('signin');

  let email = $state('');
  let password = $state('');
  let name = $state('');
</script>

<div class="min-h-screen bg-[--color-nb-white] flex items-center justify-center p-6">
  <div class="w-full max-w-md">
    <div class="mb-8">
      <h1 class="text-4xl font-black tracking-tight">PatientVault</h1>
      <p class="mt-1 text-lg font-medium text-[--color-nb-text-secondary]">Provider Portal</p>
    </div>

    <div class="flex gap-2 mb-4">
      <button
        type="button"
        class="flex-1 px-4 py-2 font-bold nb-border {mode === 'signin'
          ? 'bg-[--color-nb-accent] nb-shadow'
          : 'bg-[--color-nb-white] hover:bg-[--color-nb-surface]'}"
        onclick={() => (mode = 'signin')}
      >
        Sign in
      </button>
      <button
        type="button"
        class="flex-1 px-4 py-2 font-bold nb-border {mode === 'signup'
          ? 'bg-[--color-nb-accent] nb-shadow'
          : 'bg-[--color-nb-white] hover:bg-[--color-nb-surface]'}"
        onclick={() => (mode = 'signup')}
      >
        Create account
      </button>
    </div>

    <Card>
      <h2 class="text-2xl font-bold mb-6">{mode === 'signin' ? 'Sign in' : 'Create provider account'}</h2>

      {#if form?.error}
        <div class="mb-4 p-3 bg-[--color-nb-danger] text-[--color-nb-black] font-bold nb-border">
          {form.error}
        </div>
      {/if}

      {#if mode === 'signin'}
        <form method="POST" action="?/signin" class="flex flex-col gap-4">
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
            showToggle
            required
          />
          <Button type="submit" variant="primary" class="w-full mt-2">Sign in</Button>
        </form>
      {:else}
        <form method="POST" action="?/signup" class="flex flex-col gap-4">
          <Input
            id="name"
            name="name"
            type="text"
            label="Name"
            placeholder="Dr. Patel"
            bind:value={name}
          />
          <Input
            id="signup-email"
            name="email"
            type="email"
            label="Email"
            placeholder="you@clinic.com"
            bind:value={email}
            required
          />
          <Input
            id="signup-password"
            name="password"
            type="password"
            label="Password"
            placeholder="At least 8 characters"
            bind:value={password}
            showToggle
            required
          />
          <Button type="submit" variant="primary" class="w-full mt-2">Create account</Button>
        </form>
      {/if}
    </Card>
  </div>
</div>
