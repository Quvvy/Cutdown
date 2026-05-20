<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatBytes, formatTime } from '../lib/format';
  import type { ClipHistoryEntry } from '../lib/types';

  export let open = false;
  export let entries: ClipHistoryEntry[] = [];
  export let busyPath: string | null = null;

  const dispatch = createEventDispatcher<{
    close: void;
    reveal: { path: string };
    openClip: { path: string };
    copyPath: { path: string };
    upload: { path: string };
    remove: { path: string };
    clear: void;
  }>();

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  function formatExportedAt(value: string): string {
    const parsed = Date.parse(value);
    if (Number.isNaN(parsed)) {
      return value;
    }

    return new Date(parsed).toLocaleString();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="drawer-backdrop" on:click={() => dispatch('close')}></div>
  <aside class="history-drawer" aria-label="Clip history">
    <header>
      <h2>Clip history</h2>
      <button type="button" class="icon-button" title="Close" on:click={() => dispatch('close')}>Close</button>
    </header>

    {#if entries.length === 0}
      <p class="history-drawer__empty">Exported clips will appear here.</p>
    {:else}
      <ul class="history-drawer__list">
        {#each entries as entry (entry.outputPath)}
          <li class="history-drawer__item">
            <div class="history-drawer__meta">
              <strong>{fileName(entry.outputPath)}</strong>
              <span>{formatExportedAt(entry.exportedAt)}</span>
              <span>{entry.presetId} · {formatBytes(entry.fileSize)} · {formatTime(entry.duration)}</span>
            </div>
            <div class="history-drawer__actions">
              <button type="button" class="secondary" title="Show in Explorer" on:click={() => dispatch('reveal', { path: entry.outputPath })}>Reveal</button>
              <button type="button" class="secondary" title="Open exported clip in editor" on:click={() => dispatch('openClip', { path: entry.outputPath })}>Open</button>
              <button type="button" class="secondary" title="Copy file path to clipboard" on:click={() => dispatch('copyPath', { path: entry.outputPath })}>Copy path</button>
              <button
                type="button"
                class="secondary"
                title="Upload to Catbox and copy link"
                disabled={busyPath === entry.outputPath}
                on:click={() => dispatch('upload', { path: entry.outputPath })}
              >
                {busyPath === entry.outputPath ? 'Uploading…' : 'Upload'}
              </button>
              <button type="button" class="secondary" title="Remove from history" on:click={() => dispatch('remove', { path: entry.outputPath })}>Remove</button>
            </div>
          </li>
        {/each}
      </ul>
      <footer>
        <button type="button" class="secondary" title="Clear all clip history" on:click={() => dispatch('clear')}>Clear history</button>
      </footer>
    {/if}
  </aside>
{/if}
