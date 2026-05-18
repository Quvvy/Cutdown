<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatTime } from '../lib/format';

  export let open = false;
  export let outputPath = '';
  export let segmentCount = 0;
  export let duration = 0;

  const dispatch = createEventDispatcher<{
    close: void;
    confirm: void;
    chooseOutput: void;
  }>();
</script>

{#if open}
  <div class="modal-backdrop">
    <section class="modal" aria-label="Export clip">
      <header>
        <h2>Export sequence</h2>
        <button type="button" class="icon-button" on:click={() => dispatch('close')}>Close</button>
      </header>

      <dl>
        <div>
          <dt>Sequence</dt>
          <dd>{segmentCount} segment{segmentCount === 1 ? '' : 's'} | {formatTime(duration)}</dd>
        </div>
        <div>
          <dt>Output</dt>
          <dd>{outputPath || 'Choose an output path'}</dd>
        </div>
      </dl>

      <footer>
        <button type="button" class="secondary" on:click={() => dispatch('chooseOutput')}>Choose Output</button>
        <button type="button" disabled={!outputPath} on:click={() => dispatch('confirm')}>Export</button>
      </footer>
    </section>
  </div>
{/if}
