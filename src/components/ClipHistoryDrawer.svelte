<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatBytes, formatTime } from '../lib/format';
  import type { ClipHistoryEntry } from '../lib/types';
  import DraggablePanel from './DraggablePanel.svelte';

  export let open = false;
  export let entries: ClipHistoryEntry[] = [];
  export let busyPath: string | null = null;
  export let uploadConfigured = true;

  const dispatch = createEventDispatcher<{
    close: void;
    reveal: { path: string };
    openClip: { path: string };
    copyPath: { path: string };
    copyLink: { url: string };
    upload: { path: string };
    remove: { path: string };
    clear: void;
    openSettings: void;
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

<DraggablePanel open={open} title="Clip history" width={620} maxHeight="min(85vh, 640px)" on:close={() => dispatch('close')}>
  {#if entries.length === 0}
    <p class="history-drawer__empty">Exported clips will appear here.</p>
    {#if !uploadConfigured}
      <p class="history-drawer__empty">
        Configure an upload target in
        <button type="button" class="link-button" on:click={() => dispatch('openSettings')}>Settings → Upload</button>
      </p>
    {/if}
  {:else}
    <ul class="history-drawer__list">
      {#each entries as entry (entry.outputPath)}
        <li class="history-drawer__item">
          <div class="history-drawer__meta">
            <strong>{fileName(entry.outputPath)}</strong>
            {#if entry.shareUrl}
              <span class="history-drawer__badge">Shared</span>
            {/if}
            <span>{formatExportedAt(entry.exportedAt)}</span>
            <span>{entry.presetId} · {formatBytes(entry.fileSize)} · {formatTime(entry.duration)}</span>
          </div>
          <div class="history-drawer__actions">
            <button type="button" class="secondary" on:click={() => dispatch('reveal', { path: entry.outputPath })}>Reveal</button>
            <button type="button" class="secondary" on:click={() => dispatch('openClip', { path: entry.outputPath })}>Open</button>
            <button type="button" class="secondary" on:click={() => dispatch('copyPath', { path: entry.outputPath })}>Copy path</button>
            {#if entry.shareUrl}
              <button type="button" class="secondary" on:click={() => dispatch('copyLink', { url: entry.shareUrl ?? '' })}>Copy link</button>
            {/if}
            <button
              type="button"
              class="secondary"
              disabled={busyPath === entry.outputPath}
              on:click={() => dispatch('upload', { path: entry.outputPath })}
            >
              {busyPath === entry.outputPath ? 'Uploading…' : 'Upload'}
            </button>
            <button type="button" class="secondary" on:click={() => dispatch('remove', { path: entry.outputPath })}>Remove</button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}

  <svelte:fragment slot="footer">
    {#if entries.length > 0}
      <button type="button" class="secondary" on:click={() => dispatch('clear')}>Clear history</button>
    {/if}
  </svelte:fragment>
</DraggablePanel>
