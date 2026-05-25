<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import DraggablePanel from './DraggablePanel.svelte';
  import { formatUpdateBytes } from '../lib/appUpdate';

  export let visible = false;
  export let version = '';
  export let notes = '';
  export let currentVersion = '';
  export let installing = false;
  export let installError = '';
  export let downloadedBytes = 0;
  export let totalBytes: number | null = null;

  const dispatch = createEventDispatcher<{
    close: void;
    install: void;
  }>();

  $: progressPercent =
    totalBytes && totalBytes > 0 ? Math.min(100, Math.round((downloadedBytes / totalBytes) * 100)) : null;
</script>

<DraggablePanel open={visible} title="Update available" width={460} on:close={() => dispatch('close')}>
  <div class="panel-section">
    <p class="panel-section__lead">
      Cutdown <strong>{version}</strong> is available. You are on <strong>{currentVersion}</strong>.
    </p>

    {#if notes.trim()}
      <div class="update-modal__notes">{notes.trim()}</div>
    {/if}

    {#if installing}
      <div class="update-modal__progress" aria-live="polite">
        <div class="update-modal__progress-bar" style:width={`${progressPercent ?? 0}%`}></div>
      </div>
      <p class="modal__hint">
        {#if progressPercent != null}
          Downloading… {progressPercent}% ({formatUpdateBytes(downloadedBytes)}{totalBytes
            ? ` / ${formatUpdateBytes(totalBytes)}`
            : ''})
        {:else}
          Downloading update…
        {/if}
      </p>
      <p class="modal__hint">Cutdown will restart to finish installing the update.</p>
    {/if}

    {#if installError}
      <p class="update-modal__error">{installError}</p>
    {/if}
  </div>

  <footer class="modal__footer">
    <button type="button" class="secondary" disabled={installing} on:click={() => dispatch('close')}>
      Later
    </button>
    <button type="button" class="primary" disabled={installing} on:click={() => dispatch('install')}>
      {installing ? 'Installing…' : 'Install update'}
    </button>
  </footer>
</DraggablePanel>

<style>
  .update-modal__notes {
    background: var(--bg3);
    border: 1px solid var(--line);
    color: var(--text);
    font-size: 12px;
    line-height: 1.45;
    max-height: 180px;
    overflow: auto;
    padding: 10px 12px;
    white-space: pre-wrap;
  }

  .update-modal__progress {
    background: var(--bg3);
    border: 1px solid var(--line);
    height: 8px;
    margin-top: 10px;
    overflow: hidden;
  }

  .update-modal__progress-bar {
    background: var(--accent);
    height: 100%;
    transition: width 120ms linear;
  }

  .update-modal__error {
    color: #f87171;
    font-size: 12px;
    margin: 10px 0 0;
  }
</style>
