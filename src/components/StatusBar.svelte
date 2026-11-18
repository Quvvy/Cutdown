<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import ProgressBar from './ProgressBar.svelte';
  import { formatTime } from '../lib/format';
  import type { ExportStatus, TimelineSegment } from '../stores/editor';

  /** Length of what Export will write: kept parts, trimmed to In/Out. */
  export let exportSeconds = 0;
  export let exportParts = 0;
  export let hasClip = false;
  export let selectedSegment: TimelineSegment | null = null;
  export let range: { start: number; end: number } | null = null;
  export let exportStatus: ExportStatus;
  export let statusLabel = '';
  /** Brief confirmation of the last edit. */
  export let note = '';
  export let progressPercent: number | null = null;

  const dispatch = createEventDispatcher<{ cancel: void }>();

  $: running = exportStatus.state === 'running';
  // Finished exports are shown on the result card, so the bar only reports work in progress.
  $: label = note || (hasClip && exportStatus.state === 'idle' ? statusLabel : '');
</script>

<footer class="bottom-bar" aria-label="Editor status">
  <div class="bottom-bar__stats">
    <span class="stat" class:stat--empty={!hasClip} title="What Export will write: the kept parts, trimmed to In and Out">
      <span class="stat__label">Export</span>
      <strong>{hasClip ? formatTime(exportSeconds) : '—'}</strong>
      {#if hasClip}
        <small>{exportParts} {exportParts === 1 ? 'part' : 'parts'}</small>
      {/if}
    </span>
    <span
      class="stat"
      class:stat--empty={!selectedSegment}
      title={selectedSegment ? `${formatTime(selectedSegment.sourceStart)} – ${formatTime(selectedSegment.sourceEnd)}` : 'No part selected'}
    >
      <span class="stat__label">Selected</span>
      <strong>{selectedSegment ? formatTime(selectedSegment.sourceEnd - selectedSegment.sourceStart) : '—'}</strong>
    </span>
    <span class="stat" class:stat--empty={!range} title={range ? `In ${formatTime(range.start)} – Out ${formatTime(range.end)}` : 'No In/Out range'}>
      <span class="stat__label">In → Out</span>
      <strong>{range ? formatTime(range.end - range.start) : '—'}</strong>
    </span>
  </div>
  <div class="bottom-bar__status" aria-live="polite">
    {#if running}
      <ProgressBar active={true} label={statusLabel} percent={progressPercent} state="running" />
      <button type="button" class="secondary" on:click={() => dispatch('cancel')}>Cancel</button>
    {:else if label}
      <span class="bottom-bar__note" title={label}>{label}</span>
    {/if}
  </div>
</footer>
