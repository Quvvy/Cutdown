<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatBytes, formatTime } from '../lib/format';
  import type { ClipHistoryEntry } from '../lib/types';
  import DraggablePanel from './DraggablePanel.svelte';
  import IconButton from './IconButton.svelte';
  import { fileManagerName } from '../lib/platform';

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

  const PRESET_NAMES: Record<string, string> = {
    'lossless-trim': 'Original quality',
    discord: 'Discord',
    twitter: 'Twitter / X',
  };

  function presetName(id: string): string {
    return PRESET_NAMES[id] ?? id;
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
    <div class="history-drawer__empty">
      <p class="history-drawer__empty-title">No exports yet</p>
      <p>Clips you export show up here, so you can copy, upload or find them again later.</p>
      {#if !uploadConfigured}
        <button type="button" class="secondary" on:click={() => dispatch('openSettings')}>Set up uploads</button>
      {/if}
    </div>
  {:else}
    <ul class="history-drawer__list">
      {#each entries as entry (entry.outputPath)}
        <li class="history-drawer__item">
          <div class="history-drawer__meta">
            <div class="history-drawer__name">
              <strong title={entry.outputPath}>{fileName(entry.outputPath)}</strong>
              {#if entry.shareUrl}
                <span class="history-drawer__badge">Shared</span>
              {/if}
            </div>
            <span class="mono">{presetName(entry.presetId)} · {formatBytes(entry.fileSize)} · {formatTime(entry.duration)}</span>
            <span>{formatExportedAt(entry.exportedAt)}</span>
          </div>
          <IconButton icon="dismiss" title="Remove from history (the file stays on disk)" variant="mini" size={16} on:click={() => dispatch('remove', { path: entry.outputPath })} />
          <div class="history-drawer__actions">
            <button type="button" class="secondary" on:click={() => dispatch('openClip', { path: entry.outputPath })}>Open in editor</button>
            <button type="button" class="secondary" on:click={() => dispatch('reveal', { path: entry.outputPath })}>Show in {fileManagerName}</button>
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
