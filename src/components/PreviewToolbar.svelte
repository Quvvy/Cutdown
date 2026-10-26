<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import IconButton from './IconButton.svelte';

  type CropAspect = 'free' | '16:9' | '9:16' | '4:3' | '1:1' | 'custom';

  export let hasClip = false;
  export let proxyRunning = false;
  export let cropEnabled = false;
  export let cropLockAspect = false;
  export let cropAspect: CropAspect = 'free';
  export let cropCustomAspectW = 16;
  export let cropCustomAspectH = 9;
  export let playbackRate = 1;

  const dispatch = createEventDispatcher<{
    aspect: { aspect: CropAspect };
    fit: void;
    zoomOut: void;
    zoomIn: void;
    proxy: void;
  }>();

  const presets: Exclude<CropAspect, 'free' | 'custom'>[] = ['16:9', '9:16', '4:3', '1:1'];
  const rates = [0.5, 1, 2];
</script>

<div class="preview-panel__tools">
  <div class="preview-panel__crop">
    <IconButton
      icon="crop"
      title={cropEnabled ? 'Disable crop overlay' : 'Enable crop overlay'}
      variant="secondary"
      active={cropEnabled}
      disabled={!hasClip}
      on:click={() => (cropEnabled = !cropEnabled)}
    />
    {#if cropEnabled}
      <div class="crop-presets" role="group" aria-label="Crop aspect ratio">
        <label class="preview-panel__crop-lock" title="Lock crop aspect ratio while resizing">
          <input type="checkbox" bind:checked={cropLockAspect} />
          Lock ratio
        </label>
        {#each presets as preset}
          <button
            type="button"
            class="secondary"
            class:active={cropAspect === preset}
            title={`Crop ${preset}`}
            on:click={() => dispatch('aspect', { aspect: preset })}
          >
            {preset}
          </button>
        {/each}
        <span class="preview-panel__crop-custom">
          <input type="number" class="preview-panel__crop-input" min="1" max="100" bind:value={cropCustomAspectW} aria-label="Custom aspect width" />
          <span>:</span>
          <input type="number" class="preview-panel__crop-input" min="1" max="100" bind:value={cropCustomAspectH} aria-label="Custom aspect height" />
          <button
            type="button"
            class="secondary"
            class:active={cropAspect === 'custom'}
            title="Apply custom aspect ratio"
            on:click={() => dispatch('aspect', { aspect: 'custom' })}
          >
            Custom
          </button>
        </span>
        <button
          type="button"
          class="secondary"
          class:active={cropAspect === 'free'}
          title="Free crop (unlocked aspect)"
          on:click={() => dispatch('aspect', { aspect: 'free' })}
        >
          Free
        </button>
      </div>
    {/if}
  </div>
  <span class="preview-panel__divider" aria-hidden="true"></span>
  <IconButton icon="scaleFit" title="Fit preview to panel" variant="secondary" disabled={!hasClip} on:click={() => dispatch('fit')} />
  <button type="button" class="secondary" title="Zoom preview out" disabled={!hasClip} on:click={() => dispatch('zoomOut')}>−</button>
  <button type="button" class="secondary" title="Zoom preview in" disabled={!hasClip} on:click={() => dispatch('zoomIn')}>+</button>
  <span class="preview-panel__divider" aria-hidden="true"></span>
  {#each rates as rate}
    <button type="button" class="secondary" class:active={playbackRate === rate} disabled={!hasClip} on:click={() => (playbackRate = rate)}>{rate}×</button>
  {/each}
  <button
    type="button"
    class="secondary"
    title="Rebuild a playable H.264 preview for heavy codecs"
    disabled={!hasClip || proxyRunning}
    on:click={() => dispatch('proxy')}
  >
    Proxy
  </button>
</div>
