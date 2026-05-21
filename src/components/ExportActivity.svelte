<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import ProgressBar from './ProgressBar.svelte';

  export let visible = false;
  export let jobLabel = '';
  export let jobIndex = 1;
  export let jobTotal = 1;
  export let presetName = '';
  export let percent: number | null = null;
  export let message = '';
  export let outputPath: string | null = null;

  const dispatch = createEventDispatcher<{
    reveal: void;
  }>();

  $: queueLabel = jobTotal > 1 ? `Job ${jobIndex} of ${jobTotal}` : '';
</script>

{#if visible}
  <section class="export-activity" aria-label="Export progress">
    <div class="export-activity__main">
      <div class="export-activity__titles">
        <strong>{jobLabel || 'Exporting…'}</strong>
        {#if queueLabel}
          <span class="export-activity__queue">{queueLabel}</span>
        {/if}
        {#if presetName}
          <span class="export-activity__preset">{presetName}</span>
        {/if}
      </div>
      <ProgressBar active={true} label={message} {percent} state="running" />
    </div>
    {#if outputPath}
      <button type="button" class="secondary" title="Show exported file in Explorer" on:click={() => dispatch('reveal')}>
        Reveal folder
      </button>
    {/if}
  </section>
{/if}
