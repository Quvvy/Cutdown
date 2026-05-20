<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import {
    createCatboxProvider,
    createCustomProvider,
    createFilegardenProvider,
    kindLabel,
    newProviderId,
    parseProvidersFromSettings,
    serializeProviders,
    type CatboxConfig,
    type FilegardenConfig,
    type HttpMultipartConfig,
    type UploadProvider,
    type UploadProviderKind,
  } from '../lib/uploadProviders';
  import {
    createCustomPreset,
    modeLabel,
    parseCustomPresetsFromSettings,
    serializeCustomPresets,
    type CustomExportPreset,
  } from '../lib/exportPresets';

  export let visible = false;
  export let watchFolder: string | null = null;
  export let watchFolderEnabled = false;
  export let defaultExportDir: string | null = null;
  export let exportPresetId = 'lossless-trim';
  export let preferGpuEncoding = true;
  export let runAtStartup = false;
  export let uploadProviders: UploadProvider[] = [];
  export let defaultUploadProviderId: string | null = null;
  export let customExportPresets: CustomExportPreset[] = [];
  export let ffmpegStatus = '';
  export let gpuEncoders: string[] = [];

  let editingId: string | null = null;
  let editingPresetId: string | null = null;

  const dispatch = createEventDispatcher<{
    close: void;
    error: { message: string };
    saved: {
      watchFolder: string | null;
      watchFolderEnabled: boolean;
      defaultExportDir: string | null;
      lastPresetId: string;
      preferGpuEncoding: boolean;
      runAtStartup: boolean;
      uploadProviders: UploadProvider[];
      defaultUploadProviderId: string | null;
      customExportPresets: CustomExportPreset[];
    };
  }>();

  $: editingPreset = customExportPresets.find((preset) => preset.id === editingPresetId) ?? null;
  $: editingProvider = uploadProviders.find((provider) => provider.id === editingId) ?? null;
  $: editingCatbox =
    editingProvider?.kind === 'catbox' ? (editingProvider.config as CatboxConfig) : null;
  $: editingFilegarden =
    editingProvider?.kind === 'filegarden' ? (editingProvider.config as FilegardenConfig) : null;
  $: editingCustom =
    editingProvider?.kind === 'http_multipart' ? (editingProvider.config as HttpMultipartConfig) : null;

  function addProvider(kind: UploadProviderKind): void {
    const provider =
      kind === 'catbox'
        ? { ...createCatboxProvider(), id: newProviderId() }
        : kind === 'filegarden'
          ? { ...createFilegardenProvider(), id: newProviderId() }
          : createCustomProvider();
    uploadProviders = [...uploadProviders, provider];
    editingId = provider.id;
    if (!defaultUploadProviderId) {
      defaultUploadProviderId = provider.id;
    }
  }

  function ensureDefaultUploadProvider(): void {
    const enabled = uploadProviders.filter((provider) => provider.enabled);
    if (enabled.length === 0) {
      defaultUploadProviderId = null;
      return;
    }

    if (
      defaultUploadProviderId &&
      enabled.some((provider) => provider.id === defaultUploadProviderId)
    ) {
      return;
    }

    defaultUploadProviderId = enabled[0].id;
  }

  function addCustomPreset(): void {
    const preset = createCustomPreset();
    customExportPresets = [...customExportPresets, preset];
    editingPresetId = preset.id;
  }

  function removeCustomPreset(id: string): void {
    customExportPresets = customExportPresets.filter((preset) => preset.id !== id);
    if (editingPresetId === id) {
      editingPresetId = null;
    }
    if (exportPresetId === id) {
      exportPresetId = 'lossless-trim';
    }
  }

  function updateCustomPreset(patch: Partial<CustomExportPreset>): void {
    if (!editingPreset) {
      return;
    }

    customExportPresets = customExportPresets.map((preset) =>
      preset.id === editingPreset.id ? { ...preset, ...patch } : preset,
    );
  }

  function handlePresetModeChange(event: Event): void {
    const value = (event.currentTarget as HTMLSelectElement).value;
    if (value === 'bitrate' || value === 'crf' || value === 'target_size') {
      updateCustomPreset({ mode: value });
    }
  }

  function removeProvider(id: string): void {
    uploadProviders = uploadProviders.filter((provider) => provider.id !== id);
    if (editingId === id) {
      editingId = null;
    }
    ensureDefaultUploadProvider();
  }

  function updateHeaderField(field: 'name' | 'enabled', value: string | boolean): void {
    if (!editingProvider) {
      return;
    }

    uploadProviders = uploadProviders.map((provider) =>
      provider.id === editingProvider.id ? { ...provider, [field]: value } : provider,
    );
  }

  function updateCatboxConfig(patch: Partial<CatboxConfig>): void {
    updateProviderConfig((config) => ({ ...(config as CatboxConfig), ...patch }));
  }

  function updateFilegardenConfig(patch: Partial<FilegardenConfig>): void {
    updateProviderConfig((config) => ({ ...(config as FilegardenConfig), ...patch }));
  }

  function updateCustomConfig(patch: Partial<HttpMultipartConfig>): void {
    updateProviderConfig((config) => ({ ...(config as HttpMultipartConfig), ...patch }));
  }

  function updateProviderConfig(
    updater: (
      config: CatboxConfig | FilegardenConfig | HttpMultipartConfig,
    ) => CatboxConfig | FilegardenConfig | HttpMultipartConfig,
  ): void {
    if (!editingProvider) {
      return;
    }

    uploadProviders = uploadProviders.map((provider) =>
      provider.id === editingProvider.id ? { ...provider, config: updater(provider.config) } : provider,
    );
  }

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
    ensureDefaultUploadProvider();
    const providers = serializeProviders(uploadProviders);
    const presets = serializeCustomPresets(customExportPresets);

    try {
      const saved = await invoke<{
        watchFolder: string | null;
        watchFolderEnabled: boolean;
        defaultExportDir: string | null;
        lastPresetId: string;
        preferGpuEncoding: boolean;
        runAtStartup: boolean;
        uploadProviders: UploadProvider[];
        defaultUploadProviderId: string | null;
        customExportPresets: CustomExportPreset[];
      }>('save_editor_settings', {
        params: {
          watchFolder,
          watchFolderEnabled,
          defaultExportDir,
          lastPresetId: exportPresetId,
          preferGpuEncoding,
          runAtStartup,
          providers,
          defaultUploadProviderId,
          customExportPresets: presets,
        },
      });

      uploadProviders = parseProvidersFromSettings(saved.uploadProviders);
      defaultUploadProviderId = saved.defaultUploadProviderId;
      customExportPresets = parseCustomPresetsFromSettings(saved.customExportPresets);
      exportPresetId = saved.lastPresetId;
      ensureDefaultUploadProvider();

      dispatch('saved', {
        watchFolder: saved.watchFolder,
        watchFolderEnabled: saved.watchFolderEnabled,
        defaultExportDir: saved.defaultExportDir,
        lastPresetId: saved.lastPresetId,
        preferGpuEncoding: saved.preferGpuEncoding,
        runAtStartup: saved.runAtStartup,
        uploadProviders,
        defaultUploadProviderId,
        customExportPresets,
      });
      dispatch('close');
    } catch (error) {
      dispatch('error', {
        message: error instanceof Error ? error.message : String(error),
      });
    }
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

      <div class="modal__body">
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
          <dt>Custom export presets</dt>
          <dd class="modal__output preset-settings">
            <p class="modal__hint">
              Built-in presets stay available. Custom presets appear in the export dialog alongside them.
            </p>
            <div class="preset-settings__toolbar">
              <button type="button" class="secondary" on:click={addCustomPreset}>Add preset</button>
            </div>
            <ul class="preset-settings__list">
              {#each customExportPresets as preset (preset.id)}
                <li class:selected={preset.id === editingPresetId}>
                  <div class="preset-settings__meta">
                    <span>{preset.name}</span>
                    <small>{preset.lossless ? 'Lossless' : modeLabel(preset.mode)}</small>
                  </div>
                  <div class="preset-settings__actions">
                    <button type="button" class="secondary" on:click={() => (editingPresetId = preset.id)}>
                      Edit
                    </button>
                    <button type="button" class="secondary" on:click={() => removeCustomPreset(preset.id)}>
                      Remove
                    </button>
                  </div>
                </li>
              {/each}
            </ul>

            {#if editingPreset}
              <div class="preset-settings__editor">
                <label class="modal__stack">
                  <span>Name</span>
                  <input
                    type="text"
                    class="modal__text-input"
                    value={editingPreset.name}
                    on:input={(event) => updateCustomPreset({ name: event.currentTarget.value })}
                  />
                </label>
                <label class="modal__stack">
                  <span>Description (optional)</span>
                  <input
                    type="text"
                    class="modal__text-input"
                    value={editingPreset.description}
                    on:input={(event) => updateCustomPreset({ description: event.currentTarget.value })}
                  />
                </label>
                <label class="modal__stack">
                  <input
                    type="checkbox"
                    checked={editingPreset.lossless}
                    on:change={(event) =>
                      updateCustomPreset({ lossless: event.currentTarget.checked })}
                  />
                  Lossless stream-copy (no re-encode)
                </label>

                {#if !editingPreset.lossless}
                  <label class="modal__stack">
                    <span>Encoding mode</span>
                    <select value={editingPreset.mode} on:change={handlePresetModeChange}>
                      <option value="bitrate">Target bitrate</option>
                      <option value="crf">Quality (CRF)</option>
                      <option value="target_size">Target file size</option>
                    </select>
                  </label>

                  <label class="modal__stack">
                    <span>Encoder speed</span>
                    <select
                      value={editingPreset.encoderSpeed ?? 'fast'}
                      on:change={(event) =>
                        updateCustomPreset({ encoderSpeed: event.currentTarget.value })}
                    >
                      <option value="fast">Fast</option>
                      <option value="medium">Medium</option>
                      <option value="slow">Slow</option>
                    </select>
                  </label>

                  <label class="modal__stack">
                    <span>Audio bitrate (kbps)</span>
                    <input
                      type="number"
                      class="modal__text-input"
                      min="64"
                      max="512"
                      value={editingPreset.audioBitrateKbps ?? 128}
                      on:input={(event) =>
                        updateCustomPreset({
                          audioBitrateKbps: Number(event.currentTarget.value) || 128,
                        })}
                    />
                  </label>

                  {#if editingPreset.mode === 'bitrate'}
                    <label class="modal__stack">
                      <span>Video bitrate (kbps)</span>
                      <input
                        type="number"
                        class="modal__text-input"
                        min="300"
                        max="50000"
                        value={editingPreset.videoBitrateKbps ?? 2500}
                        on:input={(event) =>
                          updateCustomPreset({
                            videoBitrateKbps: Number(event.currentTarget.value) || 2500,
                          })}
                      />
                    </label>
                  {:else if editingPreset.mode === 'crf'}
                    <label class="modal__stack">
                      <span>CRF (lower = higher quality)</span>
                      <input
                        type="number"
                        class="modal__text-input"
                        min="16"
                        max="35"
                        value={editingPreset.crf ?? 20}
                        on:input={(event) =>
                          updateCustomPreset({ crf: Number(event.currentTarget.value) || 20 })}
                      />
                    </label>
                  {:else if editingPreset.mode === 'target_size'}
                    <label class="modal__stack">
                      <span>Target file size (bytes)</span>
                      <input
                        type="number"
                        class="modal__text-input"
                        min="1048576"
                        value={editingPreset.targetBytes ?? 9 * 1024 * 1024}
                        on:input={(event) =>
                          updateCustomPreset({
                            targetBytes: Number(event.currentTarget.value) || 9 * 1024 * 1024,
                          })}
                      />
                    </label>
                  {/if}

                  <div class="preset-settings__grid">
                    <label class="modal__stack">
                      <span>Max width (optional)</span>
                      <input
                        type="number"
                        class="modal__text-input"
                        min="0"
                        placeholder="No limit"
                        value={editingPreset.maxWidth ?? ''}
                        on:input={(event) =>
                          updateCustomPreset({
                            maxWidth: event.currentTarget.value
                              ? Number(event.currentTarget.value)
                              : null,
                          })}
                      />
                    </label>
                    <label class="modal__stack">
                      <span>Max height (optional)</span>
                      <input
                        type="number"
                        class="modal__text-input"
                        min="0"
                        placeholder="No limit"
                        value={editingPreset.maxHeight ?? ''}
                        on:input={(event) =>
                          updateCustomPreset({
                            maxHeight: event.currentTarget.value
                              ? Number(event.currentTarget.value)
                              : null,
                          })}
                      />
                    </label>
                  </div>
                {/if}
              </div>
            {/if}
          </dd>
        </div>
        <div>
          <dt>Upload targets</dt>
          <dd class="modal__output upload-settings">
            <p class="modal__hint">
              Credentials are stored locally in settings.json. Use HTTPS for custom servers. Select the filled
              circle for your default target, then click Save.
            </p>
            <div class="upload-settings__toolbar">
              <button type="button" class="secondary" on:click={() => addProvider('catbox')}>Add Catbox</button>
              <button type="button" class="secondary" on:click={() => addProvider('filegarden')}>Add File Garden</button>
              <button type="button" class="secondary" on:click={() => addProvider('http_multipart')}>Add custom server</button>
            </div>
            <ul class="upload-settings__list">
              {#each uploadProviders as provider (provider.id)}
                <li class:selected={provider.id === editingId}>
                  <label class="upload-settings__default">
                    {#if uploadProviders.filter((entry) => entry.enabled).length > 1}
                      <input
                        type="radio"
                        name="default-upload-provider"
                        checked={defaultUploadProviderId === provider.id}
                        on:change={() => (defaultUploadProviderId = provider.id)}
                      />
                    {:else if provider.enabled}
                      <span class="upload-settings__default-badge" aria-hidden="true">●</span>
                    {/if}
                    <span>{provider.name}</span>
                    <small>{kindLabel(provider.kind)}{provider.enabled ? '' : ' · disabled'}</small>
                  </label>
                  <div class="upload-settings__actions">
                    <button type="button" class="secondary" on:click={() => (editingId = provider.id)}>Edit</button>
                    <button type="button" class="secondary" on:click={() => removeProvider(provider.id)}>Remove</button>
                  </div>
                </li>
              {/each}
            </ul>

            {#if editingProvider}
              <div class="upload-settings__editor">
                <label class="modal__stack">
                  <span>Display name</span>
                  <input
                    type="text"
                    class="modal__text-input"
                    value={editingProvider.name}
                    on:input={(event) => updateHeaderField('name', event.currentTarget.value)}
                  />
                </label>
                <label class="modal__stack">
                  <input
                    type="checkbox"
                    checked={editingProvider.enabled}
                    on:change={(event) => updateHeaderField('enabled', event.currentTarget.checked)}
                  />
                  Enabled
                </label>

                {#if editingCatbox}
                  <label class="modal__stack">
                    <span>API URL</span>
                    <input
                      type="text"
                      class="modal__text-input"
                      value={editingCatbox.apiUrl}
                      on:input={(event) => updateCatboxConfig({ apiUrl: event.currentTarget.value })}
                    />
                  </label>
                  <label class="modal__stack">
                    <span>User hash (optional)</span>
                    <input
                      type="text"
                      class="modal__text-input"
                      value={editingCatbox.userHash ?? ''}
                      on:input={(event) =>
                        updateCatboxConfig({ userHash: event.currentTarget.value.trim() || null })}
                    />
                  </label>
                {:else if editingFilegarden}
                  <label class="modal__stack">
                    <span>API base URL</span>
                    <input
                      type="text"
                      class="modal__text-input"
                      placeholder="https://api.filegarden.com"
                      value={editingFilegarden.apiBase}
                      on:input={(event) => updateFilegardenConfig({ apiBase: event.currentTarget.value })}
                    />
                  </label>
                  <label class="modal__stack">
                    <span>Email</span>
                    <input
                      type="email"
                      class="modal__text-input"
                      value={editingFilegarden.email}
                      on:input={(event) => updateFilegardenConfig({ email: event.currentTarget.value })}
                    />
                  </label>
                  <label class="modal__stack">
                    <span>Password</span>
                    <input
                      type="password"
                      class="modal__text-input"
                      value={editingFilegarden.password}
                      on:input={(event) => updateFilegardenConfig({ password: event.currentTarget.value })}
                    />
                  </label>
                  <p class="modal__hint">
                    Uses the File Garden API at api.filegarden.com. TOTP / Google / Discord sign-in on the website is not
                    supported here; use password login or another upload target.
                  </p>
                  <label class="modal__stack">
                    <span>Upload URL override (optional, advanced)</span>
                    <input
                      type="text"
                      class="modal__text-input"
                      placeholder="Leave empty for default File Garden upload"
                      value={editingFilegarden.uploadUrl ?? ''}
                      on:input={(event) =>
                        updateFilegardenConfig({ uploadUrl: event.currentTarget.value.trim() || null })}
                    />
                  </label>
                {:else if editingCustom}
                  <label class="modal__stack">
                    <span>Upload URL</span>
                    <input
                      type="text"
                      class="modal__text-input"
                      value={editingCustom.url}
                      on:input={(event) => updateCustomConfig({ url: event.currentTarget.value })}
                    />
                  </label>
                  <label class="modal__stack">
                    <span>File field name</span>
                    <input
                      type="text"
                      class="modal__text-input"
                      value={editingCustom.fileField}
                      on:input={(event) => updateCustomConfig({ fileField: event.currentTarget.value })}
                    />
                  </label>
                  <label class="modal__stack">
                    <span>Authorization header (optional)</span>
                    <input
                      type="password"
                      class="modal__text-input"
                      value={editingCustom.headers.Authorization ?? ''}
                      on:input={(event) =>
                        updateCustomConfig({
                          headers: {
                            ...editingCustom.headers,
                            Authorization: event.currentTarget.value,
                          },
                        })}
                    />
                  </label>
                  <label class="modal__stack">
                    <span>Response mode</span>
                    <select
                      value={editingCustom.responseMode}
                      on:change={(event) =>
                        updateCustomConfig({
                          responseMode: event.currentTarget.value === 'json_path' ? 'json_path' : 'plain_url',
                        })}
                    >
                      <option value="plain_url">Plain URL text</option>
                      <option value="json_path">JSON path</option>
                    </select>
                  </label>
                  {#if editingCustom.responseMode === 'json_path'}
                    <label class="modal__stack">
                      <span>JSON path to URL</span>
                      <input
                        type="text"
                        class="modal__text-input"
                        placeholder="url"
                        value={editingCustom.responseJsonPath ?? ''}
                        on:input={(event) =>
                          updateCustomConfig({
                            responseJsonPath: event.currentTarget.value.trim() || null,
                          })}
                      />
                    </label>
                  {/if}
                {/if}
              </div>
            {/if}
          </dd>
        </div>
      </dl>
      </div>

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