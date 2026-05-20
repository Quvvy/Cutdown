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
  export let catboxUserHash = '';
  export let catboxApiUrl = '';
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
      catboxUserHash: string;
      catboxApiUrl: string;
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
      catboxUserHash: string | null;
      catboxApiUrl: string | null;
    }>('save_app_settings', {
      watchFolder,
      watchFolderEnabled,
      defaultExportDir,
      lastPresetId: exportPresetId,
      preferGpuEncoding,
      runAtStartup,
      catboxUserHash: catboxUserHash.trim() || null,
      catboxApiUrl: catboxApiUrl.trim() || null,
    });

    dispatch('saved', {
      watchFolder: saved.watchFolder,
      watchFolderEnabled: saved.watchFolderEnabled,
      defaultExportDir: saved.defaultExportDir,
      lastPresetId: saved.lastPresetId,
      preferGpuEncoding: saved.preferGpuEncoding,
      runAtStartup: saved.runAtStartup,
      catboxUserHash: saved.catboxUserHash ?? '',
      catboxApiUrl: saved.catboxApiUrl ?? '',
    });
    dispatch('close');
  }
</script>

{#if visible}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={() => dispatch('close')}>
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <section class="modal modal--wide" aria-label="Settings" on:click|stopPropagation>
      <header>
        <h2>Settings</h2>
        <button type="button" class="icon-button" title="Close" on:click={() => dispatch('close')}>Close</button>
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
            <button type="button" class="secondary" title="Browse for default export folder" on:click={browseExportFolder}>Browse</button>
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
        <div>
          <dt>Catbox upload</dt>
          <dd class="modal__output">
            <label class="modal__stack">
              <span>User hash (optional)</span>
              <input type="text" class="modal__text-input" bind:value={catboxUserHash} spellcheck="false" />
            </label>
            <label class="modal__stack">
              <span>API URL</span>
              <input
                type="text"
                class="modal__text-input"
                bind:value={catboxApiUrl}
                placeholder="https://catbox.moe/user/api.php"
                spellcheck="false"
              />
            </label>
          </dd>
        </div>
      </dl>

      <footer>
        <button type="button" class="secondary" title="Browse for OBS replay folder" on:click={browseWatchFolder}>Browse watch folder</button>
        <button
          type="button"
          class="secondary"
          disabled={!watchFolder}
          title="Clear watch folder selection"
          on:click={() => ((watchFolder = null), (watchFolderEnabled = false))}
        >
          Clear watch folder
        </button>
        <button type="button" title="Save settings" on:click={saveSettings}>Save</button>
      </footer>
    </section>
  </div>
{/if}
