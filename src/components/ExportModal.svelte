<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatBytes, formatTime, sanitizeExportFileName } from '../lib/format';
  import IconButton from './IconButton.svelte';

  type PresetInfo = {
    id: string;
    name: string;
    description: string;
    lossless: boolean;
    requiresGpu: boolean;
  };

  export let open = false;
  export let outputDirectory = '';
  export let outputFileName = 'cutdown.mp4';
  export let segmentCount = 0;
  export let duration = 0;
  export let rangeDuration = 0;
  export let canExportRange = false;
  export let exportMode: 'sequence' | 'range' = 'sequence';
  export let presets: PresetInfo[] = [];
  export let presetId = 'lossless-trim';

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
  $: discordEstimate =
    selectedPreset?.id === 'discord' && exportDuration > 0
      ? `Target upload size about ${formatBytes(9 * 1024 * 1024)} for ${formatTime(exportDuration)}.`
      : null;

  function handlePresetChange(event: Event): void {
    const target = event.currentTarget as HTMLSelectElement;
    dispatch('presetChange', { presetId: target.value });
  }

  function normalizeFileName(): void {
    outputFileName = sanitizeExportFileName(outputFileName);
  }
</script>

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={() => dispatch('close')}>
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <section class="modal" aria-label="Export clip" on:click|stopPropagation>
      <header>
        <h2>Export clip</h2>
        <button type="button" class="icon-button" title="Close" on:click={() => dispatch('close')}>Close</button>
      </header>

      <dl>
        <div>
          <dt>Preset</dt>
          <dd>
            <select value={presetId} on:change={handlePresetChange}>
              {#each presets as preset}
                <option value={preset.id}>{preset.name}</option>
              {/each}
            </select>
            {#if selectedPreset}
              <p class="modal__hint">{selectedPreset.description}</p>
            {/if}
            {#if discordEstimate}
              <p class="modal__hint">{discordEstimate}</p>
            {/if}
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
      </dl>

      <p class="modal__note">Existing files at the selected output path will be replaced.</p>

      <footer>
        <button type="button" class="secondary" title="Choose export folder" on:click={() => dispatch('chooseOutput')}>Choose folder</button>
        <IconButton icon="export" title="Start export" variant="primary" showLabel disabled={!canExport} on:click={() => dispatch('confirm')}>
          Export
        </IconButton>
      </footer>
    </section>
  </div>
{/if}
