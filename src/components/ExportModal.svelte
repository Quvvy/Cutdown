<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatTime } from '../lib/format';

  export let open = false;
  export let outputPath = '';
  export let segmentCount = 0;
  export let duration = 0;
  export let rangeDuration = 0;
  export let canExportRange = false;
  export let exportMode: 'sequence' | 'range' = 'sequence';

  const dispatch = createEventDispatcher<{
    close: void;
    confirm: void;
    chooseOutput: void;
    exportModeChange: { mode: 'sequence' | 'range' };
  }>();
</script>

{#if open}
  <div class="modal-backdrop">
    <section class="modal" aria-label="Export clip">
      <header>
        <h2>Export clip</h2>
        <button type="button" class="icon-button" on:click={() => dispatch('close')}>Close</button>
      </header>

      <dl>
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
          <dt>Output</dt>
          <dd>{outputPath || 'Choose an output path'}</dd>
        </div>
      </dl>

      <p class="modal__note">Existing files at the selected output path will be replaced.</p>

      <footer>
        <button type="button" class="secondary" on:click={() => dispatch('chooseOutput')}>Choose Output</button>
        <button type="button" disabled={!outputPath} on:click={() => dispatch('confirm')}>Export</button>
      </footer>
    </section>
  </div>
{/if}
