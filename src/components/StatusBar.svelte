<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import IconButton from './IconButton.svelte';
  import ProgressBar from './ProgressBar.svelte';
  import { formatTime } from '../lib/format';
  import type { ExportStatus, TimelineSegment } from '../stores/editor';

  export let outputDuration = 0;
  export let segmentCount = 0;
  export let selectedSegment: TimelineSegment | null = null;
  export let range: { start: number; end: number } | null = null;
  export let rangeLoop = false;
  export let canUseRange = false;
  /** 0..1 */
  export let volume = 1;
  export let volumeEnabled = false;
  export let exportStatus: ExportStatus;
  export let statusLabel = '';
  export let progressPercent: number | null = null;
  export let dismissible = false;
  export let shareUrl: string | null = null;

  const dispatch = createEventDispatcher<{
    toggleLoop: void;
    volume: { volume: number };
    dismiss: void;
    copyPath: void;
    copyShareLink: void;
    reveal: void;
    upload: void;
    openExport: void;
    cancel: void;
  }>();

  $: volumePercent = Math.round(volume * 100);
  $: outputPath = exportStatus.outputPath ?? null;
  $: finished = exportStatus.state === 'success' || exportStatus.state === 'error';
</script>

<footer class="bottom-bar" aria-label="Editor status">
  <div class="bottom-bar__stats">
    <span class="stat" title="Total output length after cuts">
      <span class="stat__label">Output</span>
      <strong>{formatTime(outputDuration)}</strong>
      <small>{segmentCount} seg</small>
    </span>
    <span
      class="stat"
      class:stat--empty={!selectedSegment}
      title={selectedSegment ? `${formatTime(selectedSegment.sourceStart)} - ${formatTime(selectedSegment.sourceEnd)}` : 'No segment selected'}
    >
      <span class="stat__label">Selected</span>
      <strong>{selectedSegment ? formatTime(selectedSegment.sourceEnd - selectedSegment.sourceStart) : '—'}</strong>
    </span>
    <span class="stat" class:stat--empty={!range} title={range ? `${formatTime(range.start)} - ${formatTime(range.end)}` : 'No In/Out range'}>
      <span class="stat__label">Range</span>
      <strong>{range ? formatTime(range.end - range.start) : '—'}</strong>
    </span>
  </div>
  <div class="bottom-bar__playback">
    <IconButton
      icon="loop"
      variant="secondary"
      active={rangeLoop}
      title={rangeLoop ? 'Disable range loop (Shift+L)' : 'Loop playback in I/O range (Shift+L)'}
      disabled={!canUseRange}
      on:click={() => dispatch('toggleLoop')}
    />
    <div class="bottom-bar__volume">
      <input
        id="clip-volume"
        class="app-slider"
        type="range"
        min="0"
        max="100"
        step="1"
        value={volumePercent}
        disabled={!volumeEnabled}
        aria-label="Clip volume"
        title="Clip volume"
        style:--slider-fill={`${volumePercent}%`}
        on:input={(event) => dispatch('volume', { volume: Number(event.currentTarget.value) / 100 })}
      />
      <span>{volumePercent}%</span>
    </div>
  </div>
  <div class="bottom-bar__status">
    <ProgressBar
      active={exportStatus.state === 'running'}
      label={statusLabel}
      percent={progressPercent}
      state={exportStatus.state}
      {dismissible}
      on:dismiss={() => dispatch('dismiss')}
    />
    {#if outputPath && finished}
      <div class="bottom-bar__result">
        <span class="bottom-bar__result-name" title={outputPath}>{outputPath.split(/[\\/]/).pop()}</span>
        <IconButton icon="copy" variant="secondary" title="Copy output file path" on:click={() => dispatch('copyPath')} />
        {#if shareUrl}
          <IconButton icon="link" variant="secondary" title="Copy share link" on:click={() => dispatch('copyShareLink')} />
        {/if}
        <IconButton icon="open" variant="secondary" title="Show exported file in Explorer" on:click={() => dispatch('reveal')} />
        <IconButton icon="upload" variant="secondary" title="Upload and copy share link" on:click={() => dispatch('upload')} />
        <IconButton icon="export" variant="secondary" title="Open export settings" on:click={() => dispatch('openExport')} />
      </div>
    {:else if outputPath && exportStatus.state === 'running'}
      <button type="button" class="secondary" title="Cancel export" on:click={() => dispatch('cancel')}>Cancel</button>
      <IconButton icon="open" variant="secondary" title="Show exported file in Explorer" on:click={() => dispatch('reveal')} />
    {/if}
  </div>
</footer>
