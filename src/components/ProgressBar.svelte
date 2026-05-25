<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import type { ExportStatus } from '../stores/editor';

  export let active = false;
  export let label = '';
  export let percent: number | null = null;
  export let state: ExportStatus['state'] = 'idle';
  export let dismissible = false;

  const dispatch = createEventDispatcher<{ dismiss: void }>();

  $: width = percent === null ? null : `${Math.max(0, Math.min(100, percent))}%`;
  $: showBar = active || state === 'running' || state === 'success' || state === 'error';
</script>

<div
  class="progress"
  class:progress--running={state === 'running'}
  class:progress--success={state === 'success'}
  class:progress--error={state === 'error'}
  class:progress--idle={state === 'idle'}
  class:active
  aria-live="polite"
>
  {#if showBar}
    <div
      class="progress__bar"
      class:indeterminate={active && percent === null}
      style:width={width ?? undefined}
    ></div>
  {/if}
  <span class="progress__label">{label}</span>
  {#if dismissible && (state === 'success' || state === 'error')}
    <button type="button" class="progress__dismiss" title="Dismiss" on:click={() => dispatch('dismiss')}>×</button>
  {/if}
</div>
