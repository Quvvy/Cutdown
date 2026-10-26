<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import IconButton from './IconButton.svelte';

  export let hasClip = false;
  /** Clip loaded and editable. */
  export let canEdit = false;
  export let canDeleteSegment = false;
  export let playing = false;

  const dispatch = createEventDispatcher<{
    split: void;
    deleteSegment: void;
    markIn: void;
    markOut: void;
    addMarker: void;
    togglePlay: void;
  }>();
</script>

<div class="transport-row" aria-label="Edit and playback controls">
  <div class="transport-row__group">
    <IconButton icon="split" title={canEdit ? 'Split at playhead (S)' : 'Open a clip to split'} disabled={!canEdit} on:click={() => dispatch('split')} />
    <IconButton
      icon="delete"
      title="Delete selected segment (Del)"
      variant="secondary"
      disabled={!canDeleteSegment}
      on:click={() => dispatch('deleteSegment')}
    />
  </div>
  <div class="transport-row__group">
    <IconButton icon="markIn" title="Mark In at playhead (I)" variant="secondary" disabled={!canEdit} on:click={() => dispatch('markIn')} />
    <IconButton icon="markOut" title="Mark Out at playhead (O)" variant="secondary" disabled={!canEdit} on:click={() => dispatch('markOut')} />
    <IconButton icon="bookmark" title="Add marker at playhead (M)" variant="secondary" disabled={!canEdit} on:click={() => dispatch('addMarker')} />
  </div>
  <div class="transport-row__center">
    <IconButton
      icon={playing ? 'pause' : 'play'}
      title={playing ? 'Pause (Space)' : 'Play (Space)'}
      variant="secondary"
      disabled={!hasClip}
      on:click={() => dispatch('togglePlay')}
    />
  </div>
  <span class="transport-row__hint">Right-click the timeline for range and marker tools</span>
</div>
