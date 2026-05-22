<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { kindLabel, type UploadProviderSummary } from '../lib/uploadProviders';
  import DraggablePanel from './DraggablePanel.svelte';

  export let open = false;
  export let filePath = '';
  export let providers: UploadProviderSummary[] = [];
  export let selectedProviderId: string | null = null;
  export let busy = false;

  const dispatch = createEventDispatcher<{
    close: void;
    upload: { providerId: string };
    openSettings: void;
  }>();

  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

</script>

<DraggablePanel open={open} title="Upload clip" width={440} on:close={() => dispatch('close')}>
  {#if filePath}
    <p class="panel-section__lead">Upload <strong>{fileName(filePath)}</strong> and copy the share link.</p>
  {/if}

  {#if providers.length === 0}
    <p class="modal__hint modal__hint--error">
      No enabled upload targets found. Open Settings → Upload, ensure at least one target is enabled, then click
      <strong>Save settings</strong>.
    </p>
    <button type="button" class="secondary" on:click={() => dispatch('openSettings')}>Open Settings → Upload</button>
  {:else}
    <ul class="upload-target-list" role="listbox" aria-label="Upload targets">
      {#each providers as provider (provider.id)}
        <li>
          <button
            type="button"
            class="upload-target-list__option"
            class:upload-target-list__option--selected={provider.id === selectedProviderId}
            role="option"
            aria-selected={provider.id === selectedProviderId}
            disabled={busy}
            on:click={() => (selectedProviderId = provider.id)}
          >
            <span>{provider.name}{provider.isDefault ? ' (default)' : ''}</span>
            <small>{kindLabel(provider.kind)}</small>
          </button>
        </li>
      {/each}
    </ul>
  {/if}

  <svelte:fragment slot="footer">
    <button type="button" class="secondary" on:click={() => dispatch('close')}>Cancel</button>
    <button
      type="button"
      class="primary"
      disabled={busy || providers.length === 0 || !selectedProviderId}
      on:click={() => selectedProviderId && dispatch('upload', { providerId: selectedProviderId })}
    >
      {busy ? 'Uploading…' : 'Upload and copy link'}
    </button>
  </svelte:fragment>
</DraggablePanel>
