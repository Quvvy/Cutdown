<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatTime } from '../lib/format';

  export let open = false;
  export let outputPath = '';
  export let inPoint = 0;
  export let outPoint = 0;

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
        <h2>Export lossless trim</h2>
        <button type="button" class="icon-button" on:click={() => dispatch('close')}>Close</button>
      </header>

      <dl>
        <div>
          <dt>Range</dt>
          <dd>{formatTime(inPoint)} to {formatTime(outPoint)}</dd>
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
