<script lang="ts">
  import { createEventDispatcher, onMount, tick } from 'svelte';
  import { formatBytes, formatTime } from '../lib/format';

  type Choice = {
    id: string;
    name: string;
    meta: string;
    note: string;
  };

  export let presetIds: string[] = [];
  export let presetId = 'lossless-trim';
  export let exportSeconds = 0;
  export let partCount = 0;
  export let trimmedToRange = false;
  export let sourceSeconds = 0;
  export let sourceBytes = 0;
  /** Edits that force a re-encode even for Original quality (crop, volume, …). */
  export let reencodeReasons: string[] = [];
  export let destinationName = '';
  export let destinationPath = '';

  const dispatch = createEventDispatcher<{
    choose: { presetId: string };
    export: void;
    moreOptions: void;
    close: void;
  }>();

  const DISCORD_LIMIT = 20_000_000;
  let panel: HTMLDivElement;

  $: originalBytes = sourceSeconds > 0 ? (sourceBytes * exportSeconds) / sourceSeconds : 0;
  $: choices = buildChoices(originalBytes, reencodeReasons).filter((choice) => presetIds.includes(choice.id));
  $: summary =
    partCount === 0
      ? 'Nothing to export: In and Out don’t overlap any kept part.'
      : `${formatTime(exportSeconds)} · ${partCount === 1 ? '1 part' : `${partCount} parts`}${trimmedToRange ? ', In → Out' : ''}`;

  function buildChoices(bytes: number, reasons: string[]): Choice[] {
    const lossless =
      reasons.length === 0
        ? {
            meta: `about ${formatBytes(bytes)} · instant`,
            note:
              bytes > 0 && bytes < DISCORD_LIMIT
                ? 'No quality loss, and small enough for Discord as it is. Cuts land on the nearest keyframe.'
                : 'No quality loss. Cuts land on the nearest keyframe, so a part can start a moment early.',
          }
        : {
            meta: `re-encodes · slower`,
            note: `Re-encodes because of ${reasons.join(', ')}. Full quality, but it takes a little while.`,
          };
    return [
      { id: 'lossless-trim', name: 'Original quality', ...lossless },
      {
        id: 'discord',
        name: 'Discord',
        meta: 'under 20 MB · re-encodes',
        note: 'Compressed to fit Discord’s 20 MB upload limit. Takes a few seconds per minute of video.',
      },
      {
        id: 'twitter',
        name: 'Twitter / X',
        meta: '720p · re-encodes',
        note: 'Smaller 720p H.264 that Twitter/X accepts without converting it again.',
      },
    ];
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' && partCount > 0) {
      event.preventDefault();
      dispatch('export');
    } else if (event.key === 'Escape') {
      event.preventDefault();
      dispatch('close');
    }
  }

  function handleWindowPointer(event: PointerEvent): void {
    const target = event.target as HTMLElement;
    if (!panel?.contains(target) && !target.closest('.toolbar [data-export-button]')) {
      dispatch('close');
    }
  }

  onMount(async () => {
    await tick();
    panel?.querySelector<HTMLInputElement>('input:checked')?.focus();
  });
</script>

<svelte:window on:pointerdown={handleWindowPointer} />

<!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
<div class="export-panel" role="dialog" aria-label="Export" bind:this={panel} on:keydown={handleKeydown}>
  <header class="export-panel__head">
    <h2 class="export-panel__title">Export</h2>
    <span class="export-panel__summary mono">{summary}</span>
  </header>

  <div class="export-panel__choices" role="radiogroup" aria-label="Export as">
    {#each choices as choice (choice.id)}
      <label class="export-choice" class:export-choice--selected={presetId === choice.id}>
        <input
          class="export-choice__radio"
          type="radio"
          name="export-choice"
          value={choice.id}
          checked={presetId === choice.id}
          on:change={() => dispatch('choose', { presetId: choice.id })}
        />
        <span class="export-choice__head">
          <span class="export-choice__name">{choice.name}</span>
          <span class="export-choice__meta">{choice.meta}</span>
        </span>
        <span class="export-choice__note">{choice.note}</span>
      </label>
    {/each}
  </div>

  <p class="export-panel__destination" title={destinationPath}>
    Saves next to the original as <strong class="mono">{destinationName}</strong>
  </p>

  <div class="export-panel__actions">
    <button type="button" class="tool-button" on:click={() => dispatch('moreOptions')}>More options…</button>
    <button type="button" class="primary" disabled={partCount === 0} on:click={() => dispatch('export')}>
      Export <kbd>Enter</kbd>
    </button>
  </div>
</div>
