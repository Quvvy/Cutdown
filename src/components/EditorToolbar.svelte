<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import IconButton from './IconButton.svelte';
  import type { VideoMetadata } from '../stores/editor';

  export let filePath: string | null = null;
  export let metadata: VideoMetadata | null = null;
  export let metadataDetail = '';
  export let recentSources: string[] = [];
  export let canUndo = false;
  export let canRedo = false;
  export let canExport = false;
  export let busy = false;
  export let exportPanelOpen = false;

  const dispatch = createEventDispatcher<{
    open: void;
    openProject: void;
    latestReplay: void;
    save: void;
    openRecent: { path: string };
    undo: void;
    redo: void;
    help: void;
    history: void;
    settings: void;
    export: void;
  }>();

  let fileMenuOpen = false;

  $: fileName = filePath?.split(/[\\/]/).pop() ?? '';

  function runFileAction(action: 'open' | 'openProject' | 'latestReplay' | 'save'): void {
    fileMenuOpen = false;
    dispatch(action);
  }

  function openRecent(path: string): void {
    fileMenuOpen = false;
    dispatch('openRecent', { path });
  }

  function handleWindowClick(event: MouseEvent): void {
    if (fileMenuOpen && !(event.target as HTMLElement).closest('.file-menu')) {
      fileMenuOpen = false;
    }
  }
</script>

<svelte:window on:click={handleWindowClick} />

<section class="toolbar" aria-label="Editor toolbar">
  <div class="file-menu">
    <button
      type="button"
      class="secondary file-menu__trigger"
      class:active={fileMenuOpen}
      aria-haspopup="menu"
      aria-expanded={fileMenuOpen}
      on:click|stopPropagation={() => (fileMenuOpen = !fileMenuOpen)}
    >
      File
    </button>
    {#if fileMenuOpen}
      <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
      <div
        class="file-menu__panel"
        role="menu"
        tabindex="-1"
        on:click|stopPropagation={() => {}}
        on:keydown={(event) => event.key === 'Escape' && (fileMenuOpen = false)}
      >
        <button type="button" role="menuitem" on:click={() => runFileAction('open')}><span>Open video…</span><kbd>Ctrl+O</kbd></button>
        <button type="button" role="menuitem" on:click={() => runFileAction('openProject')}><span>Open project…</span></button>
        <button type="button" role="menuitem" title="Open newest video in watch folder" on:click={() => runFileAction('latestReplay')}><span>Latest replay</span></button>
        <button type="button" role="menuitem" disabled={!filePath} on:click={() => runFileAction('save')}><span>Save project</span><kbd>Ctrl+S</kbd></button>
        <hr class="file-menu__sep" />
        <p class="file-menu__heading">Recent</p>
        {#if recentSources.length === 0}
          <p class="file-menu__empty">No recent sources yet.</p>
        {:else}
          {#each recentSources as source (source)}
            <button type="button" role="menuitem" title={source} on:click={() => openRecent(source)}>
              <span>{source.split(/[\\/]/).pop() ?? source}</span>
            </button>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
  <IconButton icon="undo" title="Undo (Ctrl+Z)" disabled={!canUndo} on:click={() => dispatch('undo')} />
  <IconButton icon="redo" title="Redo (Ctrl+Y)" disabled={!canRedo} on:click={() => dispatch('redo')} />
  <div class="toolbar__spacer"></div>
  {#if filePath}
    <span class="toolbar__clip-name" title={filePath}>{fileName}</span>
    {#if metadata}
      <span class="toolbar__meta" title={metadataDetail}>{metadata.width}×{metadata.height} · {metadata.variableFrameRate ? `VFR, up to ${Math.round(metadata.maxFps ?? metadata.fps)}` : `${metadata.fps.toFixed(0)}`} fps · {metadata.codec}</span>
    {/if}
  {/if}
  <button type="button" class="tool-button" title="Help — shortcuts and features (?)" on:click={() => dispatch('help')}>?</button>
  <IconButton icon="history" title="Clip history" on:click={() => dispatch('history')} />
  <IconButton icon="settings" title="Settings" on:click={() => dispatch('settings')} />
  <span data-export-button>
    <IconButton
      icon="export"
      title={canExport ? 'Export clip (Ctrl+E)' : 'Open a clip before exporting'}
      variant="primary"
      showLabel
      pressed={exportPanelOpen}
      disabled={!canExport || busy}
      on:click={() => dispatch('export')}>Export</IconButton
    >
  </span>
</section>
