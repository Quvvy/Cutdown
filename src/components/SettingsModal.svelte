<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';

  export let visible = false;
  export let watchFolder: string | null = null;
  export let watchFolderEnabled = false;
  export let ffmpegStatus = '';

  const dispatch = createEventDispatcher<{
    close: void;
    saved: { watchFolder: string | null; watchFolderEnabled: boolean };
  }>();

  async function browseWatchFolder(): Promise<void> {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: 'Choose OBS replay buffer folder',
    });

    if (typeof selected === 'string') {
      watchFolder = selected;
      watchFolderEnabled = true;
    }
  }

  async function saveSettings(): Promise<void> {
    const saved = await invoke<{
      watchFolder: string | null;
      watchFolderEnabled: boolean;
    }>('update_watch_folder', {
      path: watchFolder,
      enabled: watchFolderEnabled,
    });

    dispatch('saved', {
      watchFolder: saved.watchFolder,
      watchFolderEnabled: saved.watchFolderEnabled,
    });
    dispatch('close');
  }
</script>

{#if visible}
  <div class="modal-backdrop">
    <section class="modal" aria-label="Settings">
      <header>
        <h2>Settings</h2>
        <button type="button" class="icon-button" on:click={() => dispatch('close')}>Close</button>
      </header>

      <dl>
        <div>
          <dt>ffmpeg</dt>
          <dd>{ffmpegStatus || 'Checking ffmpeg availability...'}</dd>
        </div>
        <div>
          <dt>Watch folder</dt>
          <dd class="modal__mode">
            <label>
              <input type="checkbox" bind:checked={watchFolderEnabled} disabled={!watchFolder} />
              Enable watch folder notifications
            </label>
            <span>{watchFolder || 'No folder selected'}</span>
          </dd>
        </div>
      </dl>

      <footer>
        <button type="button" class="secondary" on:click={browseWatchFolder}>Browse folder</button>
        <button type="button" class="secondary" disabled={!watchFolder} on:click={() => (watchFolder = null, (watchFolderEnabled = false))}>
          Clear
        </button>
        <button type="button" on:click={saveSettings}>Save</button>
      </footer>
    </section>
  </div>
{/if}
