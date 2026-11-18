<script lang="ts">
  import { createEventDispatcher, onDestroy } from 'svelte';
  import SvgIcon from './SvgIcon.svelte';
  import IconButton from './IconButton.svelte';
  import { appIcons } from '../lib/appIcons';
  import { formatBytes } from '../lib/format';
  import { fileManagerName } from '../lib/platform';
  import type { ExportStatus } from '../stores/editor';

  export let status: ExportStatus;
  export let shareUrl: string | null = null;
  export let canRetry = false;

  const dispatch = createEventDispatcher<{
    copyFile: void;
    copyPath: void;
    copyShareLink: void;
    reveal: void;
    upload: void;
    openExport: void;
    retry: void;
    dismiss: void;
  }>();

  let copied = false;
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;

  $: success = status.state === 'success';
  $: fileName = status.outputPath?.split(/[\\/]/).pop() ?? '';
  $: folder = status.outputPath?.replace(/[\\/][^\\/]*$/, '') ?? '';
  // A new export result resets the "Copied" confirmation.
  $: status.outputPath, (copied = false);

  function copyFile(): void {
    dispatch('copyFile');
    copied = true;
    if (copiedTimer) {
      clearTimeout(copiedTimer);
    }
    copiedTimer = setTimeout(() => (copied = false), 2500);
  }

  onDestroy(() => {
    if (copiedTimer) {
      clearTimeout(copiedTimer);
    }
  });
</script>

<section
  class="export-result"
  class:export-result--error={!success}
  role={success ? 'status' : 'alert'}
  aria-label={success ? 'Export finished' : 'Export failed'}
>
  <header class="export-result__head">
    <span class="export-result__icon">
      <SvgIcon icon={success ? appIcons.success : appIcons.error} size={18} />
    </span>
    <h2 class="export-result__title">{success ? 'Clip exported' : 'Export failed'}</h2>
    <IconButton icon="dismiss" title="Dismiss" variant="mini" size={16} on:click={() => dispatch('dismiss')} />
  </header>

  {#if success}
    <p class="export-result__file" title={status.outputPath}>
      <span class="export-result__name mono">{fileName}</span>
      {#if status.outputSize}
        <span class="export-result__size mono">{formatBytes(status.outputSize)}</span>
      {/if}
    </p>
    <p class="export-result__folder" title={folder}>in {folder}</p>

    <button type="button" class="primary export-result__copy" on:click={copyFile}>
      <SvgIcon icon={copied ? appIcons.success : appIcons.copy} size={16} />
      <span>{copied ? 'Copied: paste with Ctrl+V' : 'Copy file for Discord'}</span>
    </button>

    <div class="export-result__actions">
      <button type="button" class="secondary" on:click={() => dispatch('reveal')}>
        <SvgIcon icon={appIcons.open} size={16} />Show in {fileManagerName}
      </button>
      <button type="button" class="secondary" on:click={() => dispatch('upload')}>
        <SvgIcon icon={appIcons.upload} size={16} />Upload
      </button>
      {#if shareUrl}
        <IconButton icon="link" variant="secondary" title="Copy share link" on:click={() => dispatch('copyShareLink')} />
      {/if}
      <IconButton icon="copyText" variant="secondary" title="Copy file path" on:click={() => dispatch('copyPath')} />
    </div>
  {:else}
    <p class="export-result__message">{status.message || 'Something went wrong while exporting.'}</p>
    <div class="export-result__actions">
      {#if canRetry}
        <button type="button" class="primary" on:click={() => dispatch('retry')}>
          <SvgIcon icon={appIcons.retry} size={16} />Try again
        </button>
      {/if}
      <button type="button" class="secondary" on:click={() => dispatch('openExport')}>
        <SvgIcon icon={appIcons.options} size={16} />Export options…
      </button>
    </div>
  {/if}
</section>
