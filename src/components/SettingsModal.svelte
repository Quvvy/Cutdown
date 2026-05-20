<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';

  export let visible = false;
  export let watchFolder: string | null = null;
  export let watchFolderEnabled = false;
  export let defaultExportDir: string | null = null;
  export let exportPresetId = 'lossless-trim';
  export let preferGpuEncoding = true;
  export let runAtStartup = false;
  export let ffmpegStatus = '';
  export let gpuEncoders: string[] = [];

  const dispatch = createEventDispatcher<{
    close: void;
    saved: {
      watchFolder: string | null;
      watchFolderEnabled: boolean;
      defaultExportDir: string | null;
      lastPresetId: string;
      preferGpuEncoding: boolean;
      runAtStartup: boolean;
    };
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

  async function browseExportFolder(): Promise<void> {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: 'Choose default export folder',
    });

    if (typeof selected === 'string') {
      defaultExportDir = selected;
    }
  }

  async function saveSettings(): Promise<void> {
    const saved = await invoke<{
      watchFolder: string | null;
      watchFolderEnabled: boolean;
      defaultExportDir: string | null;
      lastPresetId: string;
      preferGpuEncoding: boolean;
      runAtStartup: boolean;
    }>('save_app_settings', {
      watchFolder,
      watchFolderEnabled,
      defaultExportDir,
      lastPresetId: exportPresetId,
      preferGpuEncoding,
      runAtStartup,
    });

    dispatch('saved', {
      watchFolder: saved.watchFolder,
      watchFolderEnabled: saved.watchFolderEnabled,
      defaultExportDir: saved.defaultExportDir,
      lastPresetId: saved.lastPresetId,
      preferGpuEncoding: saved.preferGpuEncoding,
      runAtStartup: saved.runAtStartup,
    });
    dispatch('close');
  }
</script>

{#if visible}
  <div class="modal-backdrop">
    <section class="modal modal--wide" aria-label="Settings">
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
          <dt>GPU encoders</dt>
          <dd>{gpuEncoders.length > 0 ? gpuEncoders.join(', ') : 'None detected (libx264 fallback)'}</dd>
        </div>
        <div>
          <dt>Encoding</dt>
          <dd class="modal__mode">
            <label>
              <input type="checkbox" bind:checked={preferGpuEncoding} />
              Prefer GPU encoding when available
            </label>
          </dd>
        </div>
        <div>
          <dt>Default export folder</dt>
          <dd class="modal__mode">
            <span>{defaultExportDir || 'Same folder as source clip'}</span>
            <button type="button" class="secondary" on:click={browseExportFolder}>Browse</button>
          </dd>
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
        <div>
          <dt>Windows</dt>
          <dd class="modal__mode">
            <label>
              <input type="checkbox" bind:checked={runAtStartup} />
              Start Cutdown when Windows starts
            </label>
          </dd>
        </div>
      </dl>

      <footer>
        <button type="button" class="secondary" on:click={browseWatchFolder}>Browse watch folder</button>
        <button type="button" class="secondary" disabled={!watchFolder} on:click={() => ((watchFolder = null), (watchFolderEnabled = false))}>
          Clear watch folder
        </button>
        <button type="button" on:click={saveSettings}>Save</button>
      </footer>
    </section>
  </div>
{/if}
