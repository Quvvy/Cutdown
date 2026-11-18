<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import IconButton from './IconButton.svelte';

  export let hasClip = false;
  export let proxyRunning = false;
  export let cropEnabled = false;
  export let playbackRate = 1;

  const dispatch = createEventDispatcher<{
    fit: void;
    proxy: void;
  }>();

  // Same steps as the J/L shuttle.
  const rates = [0.5, 1, 1.5, 2];

  function cycleRate(): void {
    const index = rates.indexOf(playbackRate);
    playbackRate = rates[(index + 1) % rates.length] ?? 1;
  }
</script>

<div class="preview-tools" role="group" aria-label="Preview">
  <IconButton
    icon="crop"
    title={cropEnabled ? 'Turn crop off' : 'Crop the export'}
    active={cropEnabled}
    disabled={!hasClip}
    on:click={() => (cropEnabled = !cropEnabled)}
  />
  <IconButton icon="scaleFit" title="Fit preview to panel (scroll on the video to zoom)" disabled={!hasClip} on:click={() => dispatch('fit')} />
  <button
    type="button"
    class="tool-button preview-tools__speed mono"
    class:active={playbackRate !== 1}
    title="Playback speed: click to change (J/L also change it)"
    disabled={!hasClip}
    on:click={cycleRate}
  >
    {playbackRate}×
  </button>
  <button
    type="button"
    class="tool-button"
    title="Preview looks black or stutters? Make a smooth preview copy. Export still uses the original."
    disabled={!hasClip || proxyRunning}
    on:click={() => dispatch('proxy')}
  >
    Fix preview
  </button>
</div>
