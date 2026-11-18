<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  type CropAspect = 'free' | '16:9' | '9:16' | '4:3' | '1:1' | 'custom';

  export let cropLockAspect = false;
  export let cropAspect: CropAspect = 'free';
  export let cropCustomAspectW = 16;
  export let cropCustomAspectH = 9;

  const dispatch = createEventDispatcher<{
    aspect: { aspect: CropAspect };
  }>();

  const presets: Exclude<CropAspect, 'custom'>[] = ['free', '16:9', '9:16', '4:3', '1:1'];
</script>

<!-- Shown over the video while crop is on, so the ratio choices sit next to what they change. -->
<div class="crop-bar" role="group" aria-label="Crop aspect ratio">
  <span class="crop-bar__label">Crop</span>
  <div class="crop-bar__presets">
    {#each presets as preset}
      <button
        type="button"
        class="crop-bar__chip"
        class:active={cropAspect === preset}
        aria-pressed={cropAspect === preset}
        title={preset === 'free' ? 'Free crop (any shape)' : `Crop to ${preset}`}
        on:click={() => dispatch('aspect', { aspect: preset })}
      >
        {preset === 'free' ? 'Free' : preset}
      </button>
    {/each}
  </div>
  <span class="crop-bar__custom">
    <input type="number" class="preview-panel__crop-input mono" min="1" max="100" bind:value={cropCustomAspectW} aria-label="Custom ratio width" />
    <span aria-hidden="true">:</span>
    <input type="number" class="preview-panel__crop-input mono" min="1" max="100" bind:value={cropCustomAspectH} aria-label="Custom ratio height" />
    <button
      type="button"
      class="crop-bar__chip"
      class:active={cropAspect === 'custom'}
      aria-pressed={cropAspect === 'custom'}
      title="Apply custom ratio"
      on:click={() => dispatch('aspect', { aspect: 'custom' })}
    >
      Set
    </button>
  </span>
  <label class="crop-bar__lock" title="Keep the ratio while resizing">
    <input type="checkbox" bind:checked={cropLockAspect} />
    Lock ratio
  </label>
</div>
