<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatBytes, formatTime, sanitizeExportFileName } from '../lib/format';
  import type { ExportPresetInfo } from '../lib/exportPresets';
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

  const dispatch = createEventDispatcher<{
    close: void;
    confirm: void;
    chooseOutput: void;
    exportModeChange: { mode: 'sequence' | 'range' };
    presetChange: { presetId: string };
  }>();

  $: selectedPreset = presets.find((preset) => preset.id === presetId) ?? null;
  $: exportDuration = exportMode === 'range' && canExportRange ? rangeDuration : duration;
  $: canExport = Boolean(outputFileName.trim()) && Boolean(outputDirectory);
  $: canBatchPerSegment = exportMode === 'sequence' && segmentCount > 1;
  $: trimHint = usesStreamCopy
    ? 'Lossless stream-copy is active: fastest export, cuts land on nearest keyframes.'
    : streamCopyBlockers.length > 0
      ? `Stream-copy disabled because of: ${streamCopyBlockers.join(', ')}. Export will re-encode.`
      : 'This preset re-encodes video for size or quality targets.';
  $: sizeEstimate =
    selectedPreset?.targetBytes && exportDuration > 0
      ? `Target upload size about ${formatBytes(selectedPreset.targetBytes)} for ${formatTime(exportDuration)}.`
      : null;

  function selectPreset(id: string): void {
    dispatch('presetChange', { presetId: id });
  }

  function normalizeFileName(): void {
    outputFileName = sanitizeExportFileName(outputFileName);
  }
</script>

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={() => dispatch('close')}>
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <section class="modal modal--wide modal--export" aria-label="Export clip" on:click|stopPropagation>
      <header>
        <h2>Export clip</h2>
        <button type="button" class="icon-button" title="Close" on:click={() => dispatch('close')}>Close</button>
      </header>

      <div class="modal__body">
        <dl>
          <div>
            <dt>Preset</dt>
            <dd>
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
            </dd>
          </div>
          <div>
            <dt>Batch</dt>
            <dd class="modal__mode">
              <label>
                <input
                  type="checkbox"
                  bind:checked={batchPerSegment}
                  disabled={!canBatchPerSegment}
                />
                Export each kept segment as its own file
              </label>
              <label>
                <input type="checkbox" bind:checked={queueUploadAfterExport} />
                Queue upload after each export completes
              </label>
            </dd>
          </div>
          <div>
            <dt>Export mode</dt>
            <dd class="modal__mode">
              <label>
                <input
                  type="radio"
                  name="export-mode"
                  value="sequence"
                  checked={exportMode === 'sequence'}
                  on:change={() => dispatch('exportModeChange', { mode: 'sequence' })}
                />
                Sequence ({segmentCount} segment{segmentCount === 1 ? '' : 's'}, {formatTime(duration)})
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
                I/O range ({formatTime(rangeDuration)})
              </label>
            </dd>
          </div>
          <div>
            <dt>File name</dt>
            <dd class="modal__output">
              <input
                type="text"
                class="modal__text-input"
                bind:value={outputFileName}
                spellcheck="false"
                aria-label="Exported file name"
                on:blur={normalizeFileName}
              />
              <p class="modal__hint">
                {#if outputDirectory}
                  Save to: {outputDirectory}
                {:else}
                  Choose a folder with the button below.
                {/if}
              </p>
            </dd>
          </div>
          <div>
            <dt>Trim quality</dt>
            <dd class="modal__mode">
              <p class="modal__hint">{trimHint}</p>
              <label>
                <input type="checkbox" bind:checked={accurateTrim} />
                Accurate trim (re-encode segment boundaries for frame-perfect in/out)
              </label>
            </dd>
          </div>
          <div>
            <dt>Audio fades</dt>
            <dd class="modal__mode modal__mode--fades">
              <label>
                <span>Fade in (s)</span>
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
                <span>Fade out (s)</span>
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
            </dd>
          </div>
          <div>
            <dt>Audio</dt>
            <dd class="modal__mode">
              <label>
                <input type="checkbox" bind:checked={stripAudio} disabled={!hasAudio} />
                Strip audio (export video only)
              </label>
              {#if !hasAudio}
                <p class="modal__hint">This clip has no audio track.</p>
              {/if}
            </dd>
          </div>
        </dl>

        <p class="modal__note">Existing files at the selected output path will be replaced.</p>
      </div>

      <footer>
        <button type="button" class="secondary" title="Choose export folder" on:click={() => dispatch('chooseOutput')}>
          Choose folder
        </button>
        <IconButton icon="export" title="Start export" variant="primary" showLabel disabled={!canExport} on:click={() => dispatch('confirm')}>
          Export
        </IconButton>
      </footer>
    </section>
  </div>
{/if}
