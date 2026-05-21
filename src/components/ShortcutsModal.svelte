<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let open = false;

  const dispatch = createEventDispatcher<{ close: void }>();

  const shortcuts = [
    { keys: 'S', action: 'Split at playhead' },
    { keys: 'M', action: 'Add timeline marker at playhead' },
    { keys: ', / .', action: 'Previous / next marker' },
    { keys: 'Shift + M', action: 'Remove nearest marker at playhead' },
    { keys: 'Del', action: 'Delete selected marker (when selected) or segment' },
    { keys: 'I', action: 'Set range in point' },
    { keys: 'O', action: 'Set range out point' },
    { keys: 'Z', action: 'Zoom timeline to I/O range' },
    { keys: 'Shift + L', action: 'Toggle loop in I/O range' },
    { keys: 'Space', action: 'Play / pause preview' },
    { keys: 'J / K / L', action: 'Step back 1s / pause / step forward 1s' },
    { keys: '[ / ]', action: 'Snap playhead to range in / out' },
    { keys: 'Left / Right', action: 'Step one frame' },
    { keys: 'Shift + Left / Right', action: 'Step 5 seconds' },
    { keys: 'Ctrl + D', action: 'Duplicate selected segment' },
    { keys: 'Delete / Backspace', action: 'Delete selected segment' },
    { keys: 'Ctrl + Z', action: 'Undo segment edit' },
    { keys: 'Ctrl + Y', action: 'Redo segment edit' },
    { keys: 'Ctrl + Shift + Z', action: 'Redo segment edit' },
    { keys: 'Escape', action: 'Close open panel or modal' },
    { keys: '?', action: 'Open this shortcut list' },
  ];
</script>

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={() => dispatch('close')}>
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <section class="modal modal--shortcuts" aria-label="Keyboard shortcuts" on:click|stopPropagation>
      <header>
        <h2>Keyboard shortcuts</h2>
        <button type="button" class="icon-button" title="Close" on:click={() => dispatch('close')}>Close</button>
      </header>

      <table class="shortcuts-table">
        <thead>
          <tr>
            <th scope="col">Key</th>
            <th scope="col">Action</th>
          </tr>
        </thead>
        <tbody>
          {#each shortcuts as row}
            <tr>
              <td><kbd>{row.keys}</kbd></td>
              <td>{row.action}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </section>
  </div>
{/if}
