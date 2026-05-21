<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatBytes, formatTime, sanitizeExportFileName } from '../lib/format';
  import type { ExportPresetInfo } from '../lib/exportPresets';
  import DraggablePanel from './DraggablePanel.svelte';
  import IconButton from './IconButton.svelte';

  export let open = false;
  export let outputDirectory = '';
  export let outputFileName = 'cutdown.mp4';
  export let segmentCount = 0;
  export let duration = 0;
  export let rangeDuration = 0;
  export let canExportRange = false;
  export let exportMode: 'sequence' | 'range' = 'sequence';
  export let presets: ExportPresetInfo[] = [];
  export let presetId = 'lossless-trim';
  export let accurateTrim = false;
  export let stripAudio = false;
  export let hasAudio = true;
  export let batchPerSegment = false;
  export let queueUploadAfterExport = false;
  export let fadeInSeconds = 0;
  export let fadeOutSeconds = 0;
  export let usesStreamCopy = true;
  export let streamCopyBlockers: string[] = [];
  export let exportBusy = false;
  export let uploadTargetsConfigured = false;

  let activeTab: 'destination' | 'preset' | 'options' = 'destination';

  const tabs = [
    { id: 'destination' as const, label: 'Where to save' },
    { id: 'preset' as const, label: 'Preset' },
    { id: 'options' as const, label: 'Options' },
  ];

  const dispatch = createEventDispatcher<{
    close: void;
    confirm: void;
    chooseOutput: void;
    exportModeChange: { mode: 'sequence' | 'range' };
    presetChange: { presetId: string };
    openUploadSettings: void;
  }>();

  $: selectedPreset = presets.find((preset) => preset.id === presetId) ?? null;
  $: exportDuration = exportMode === 'range' && canExportRange ? rangeDuration : duration;
  $: canExport = Boolean(outputFileName.trim()) && Boolean(outputDirectory) && !exportBusy;
  $: canBatchPerSegment = exportMode === 'sequence' && segmentCount > 1;
  $: batchCount = batchPerSegment && canBatchPerSegment ? segmentCount : 1;
  $: trimHint = usesStreamCopy
    ? 'Lossless stream-copy is fastest; cuts land on nearest keyframes.'
    : streamCopyBlockers.length > 0
      ? `Re-encode required: ${streamCopyBlockers.join(', ')}.`
      : 'This preset re-encodes for size or quality targets.';
  $: sizeEstimate =
    selectedPreset?.targetBytes && exportDuration > 0
      ? `Target size about ${formatBytes(selectedPreset.targetBytes)} for ${formatTime(exportDuration)}.`
      : null;
  $: validationErrors = [
    !outputDirectory.trim() ? 'Choose an export folder on the Where to save tab.' : null,
    !outputFileName.trim() ? 'Enter a file name.' : null,
  ].filter((value): value is string => Boolean(value));

  function selectPreset(id: string): void {
    dispatch('presetChange', { presetId: id });
  }

  function normalizeFileName(): void {
    outputFileName = sanitizeExportFileName(outputFileName);
  }
</script>

<DraggablePanel open={open} title="Export clip" width={560} on:close={() => dispatch('close')}>
  {#if validationErrors.length > 0}
    <ul class="modal__validation">
      {#each validationErrors as error}
        <li class="modal__hint modal__hint--error">{error}</li>
      {/each}
    </ul>
  {/if}

  <nav class="panel-nav" aria-label="Export sections">
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        class="panel-nav__tab"
        class:panel-nav__tab--active={activeTab === tab.id}
        on:click={() => (activeTab = tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </nav>

  {#if activeTab === 'destination'}
    <div class="panel-section">
      <h3 class="panel-section__title">Output location</h3>
      <p class="panel-section__lead">Pick a folder and name for the exported file.</p>
      <div class="panel-field">
        <span>File name</span>
        <input
          type="text"
          class="modal__text-input"
          bind:value={outputFileName}
          spellcheck="false"
          aria-label="Exported file name"
          on:blur={normalizeFileName}
        />
      </div>
      <div class="panel-field">
        <span>Folder</span>
        <div class="panel-field__row">
          <span class="panel-field__path">{outputDirectory || 'Not selected yet'}</span>
          <button type="button" class="secondary" on:click={() => dispatch('chooseOutput')}>Browse</button>
        </div>
      </div>
      {#if batchCount > 1 && outputDirectory}
        <p class="modal__hint">Will export {batchCount} files into {outputDirectory}.</p>
      {/if}
      <div class="panel-field">
        <span>What to export</span>
        <div class="modal__mode">
          <label>
            <input
              type="radio"
              name="export-mode"
              value="sequence"
              checked={exportMode === 'sequence'}
              on:change={() => dispatch('exportModeChange', { mode: 'sequence' })}
            />
            All kept segments ({segmentCount} segment{segmentCount === 1 ? '' : 's'}, {formatTime(duration)})
          </label>
          <label>
            <input
              type="radio"
              name="export-mode"
              value="range"
              checked={exportMode === 'range'}
              disabled={!canExportRange}
              on:change={() => dispatch('exportModeChange', { mode: 'range' })}
            />
            I/O range only ({formatTime(rangeDuration)})
          </label>
        </div>
      </div>
      <div class="panel-field">
        <span>Batch</span>
        <label class="modal__mode">
          <input type="checkbox" bind:checked={batchPerSegment} disabled={!canBatchPerSegment} />
          Export each kept segment as its own file
        </label>
      </div>
      <p class="modal__note">Existing files at the output path will be replaced.</p>
    </div>
  {:else if activeTab === 'preset'}
    <div class="panel-section">
      <h3 class="panel-section__title">Compression preset</h3>
      <p class="panel-section__lead">Lossless is fastest for quick trims. Re-encode presets target upload size or quality.</p>
      <ul class="preset-picker" role="listbox" aria-label="Export presets">
        {#each presets as preset (preset.id)}
          <li>
            <button
              type="button"
              class="preset-picker__option"
              class:preset-picker__option--selected={preset.id === presetId}
              role="option"
              aria-selected={preset.id === presetId}
              on:click={() => selectPreset(preset.id)}
            >
              <span class="preset-picker__name">
                {preset.name}
                {#if preset.custom}
                  <small>Custom</small>
                {/if}
              </span>
              <span class="preset-picker__description">{preset.description}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if sizeEstimate}
        <p class="modal__hint">{sizeEstimate}</p>
      {/if}
      <div class="panel-info">{trimHint}</div>
    </div>
  {:else}
    <div class="panel-section">
      <h3 class="panel-section__title">Export options</h3>
      <p class="panel-section__lead">Fine-tune trim accuracy, audio, and post-export upload.</p>
      <label class="modal__mode">
        <input type="checkbox" bind:checked={accurateTrim} />
        Accurate trim (frame-perfect in/out, slower)
      </label>
      <div class="panel-field">
        <span>Audio</span>
        <label class="modal__mode">
          <input type="checkbox" bind:checked={stripAudio} disabled={!hasAudio} />
          Strip audio (video only)
        </label>
        {#if !hasAudio}
          <p class="modal__hint">This clip has no audio track.</p>
        {/if}
      </div>
      <div class="panel-field">
        <span>Fades (seconds)</span>
        <div class="modal__mode modal__mode--fades">
          <label>
            <span>In</span>
            <input
              class="modal__number-input"
              type="number"
              min="0"
              max="10"
              step="0.1"
              bind:value={fadeInSeconds}
              disabled={stripAudio || !hasAudio}
            />
          </label>
          <label>
            <span>Out</span>
            <input
              class="modal__number-input"
              type="number"
              min="0"
              max="10"
              step="0.1"
              bind:value={fadeOutSeconds}
              disabled={stripAudio || !hasAudio}
            />
          </label>
        </div>
      </div>
      <div class="panel-field">
        <span>After export</span>
        <label class="modal__mode">
          <input type="checkbox" bind:checked={queueUploadAfterExport} disabled={!uploadTargetsConfigured} />
          Upload each file and copy the share link
        </label>
        {#if queueUploadAfterExport && !uploadTargetsConfigured}
          <p class="modal__hint modal__hint--error">
            No upload targets configured.
            <button type="button" class="link-button" on:click={() => dispatch('openUploadSettings')}>Open Settings → Upload</button>
          </p>
        {/if}
      </div>
    </div>
  {/if}

  <svelte:fragment slot="footer">
    <button type="button" class="secondary" on:click={() => dispatch('close')}>Cancel</button>
    <IconButton icon="export" title="Start export" variant="primary" showLabel disabled={!canExport} on:click={() => dispatch('confirm')}>
      {exportBusy ? 'Exporting…' : 'Start export'}
    </IconButton>
  </svelte:fragment>
</DraggablePanel>
