<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import IconButton from './IconButton.svelte';
  import SvgIcon from './SvgIcon.svelte';
  import { appIcons } from '../lib/appIcons';
  import { formatTime } from '../lib/format';
  import { MAX_ZOOM } from '../lib/timelineZoom';

  export let hasClip = false;
  /** Clip loaded and editable. */
  export let canEdit = false;
  export let canDeleteSegment = false;
  export let playing = false;
  export let currentTime = 0;
  export let duration = 0;
  export let zoom = 0;
  /** In and Out are set and narrower than the clip. */
  export let rangeNarrowed = false;
  export let canUseRange = false;
  export let rangeLoop = false;
  /** 0..1 */
  export let volume = 1;
  export let volumeEnabled = false;

  const dispatch = createEventDispatcher<{
    split: void;
    deleteSegment: void;
    markIn: void;
    markOut: void;
    trimToRange: void;
    addMarker: void;
    togglePlay: void;
    toggleLoop: void;
    volume: { volume: number };
    zoomChange: { zoom: number };
    zoomFit: void;
    zoomRange: void;
  }>();

  $: volumePercent = Math.round(volume * 100);
</script>

<div class="transport-row" role="toolbar" aria-label="Edit and playback controls">
  <div class="transport-row__side">
    <div class="transport-row__group">
      <IconButton icon="split" title={canEdit ? 'Cut at playhead (X)' : 'Open a clip to cut'} disabled={!canEdit} on:click={() => dispatch('split')} />
      <IconButton icon="delete" title="Delete selected part (Del)" disabled={!canDeleteSegment} on:click={() => dispatch('deleteSegment')} />
    </div>
    <span class="transport-row__divider" aria-hidden="true"></span>
    <div class="transport-row__group">
      <IconButton icon="markIn" title="Set In at playhead (I)" disabled={!canEdit} on:click={() => dispatch('markIn')} />
      <IconButton icon="markOut" title="Set Out at playhead (O)" disabled={!canEdit} on:click={() => dispatch('markOut')} />
      <IconButton
        icon="keepRange"
        title="Trim to In → Out: remove everything outside the range"
        disabled={!canEdit || !rangeNarrowed}
        on:click={() => dispatch('trimToRange')}
      />
      <IconButton icon="bookmark" title="Add marker at playhead (M)" disabled={!canEdit} on:click={() => dispatch('addMarker')} />
    </div>
  </div>

  <div class="transport-row__center">
    <IconButton
      icon="loop"
      active={rangeLoop}
      title={rangeLoop ? 'Stop looping In → Out (Shift+L)' : 'Loop In → Out (Shift+L)'}
      disabled={!canUseRange}
      on:click={() => dispatch('toggleLoop')}
    />
    <IconButton
      icon={playing ? 'pause' : 'play'}
      title={playing ? 'Pause (Space)' : 'Play (Space)'}
      variant="secondary"
      size={20}
      disabled={!hasClip}
      on:click={() => dispatch('togglePlay')}
    />
    <span class="transport-row__time mono" aria-label="Playhead position" title="Playhead / clip length">
      <strong>{formatTime(currentTime)}</strong>
      <span>/ {formatTime(duration)}</span>
    </span>
  </div>

  <div class="transport-row__side transport-row__side--end">
    <label class="transport-row__volume" title="Clip volume (also applied to the export)">
      <SvgIcon icon={appIcons.volume} size={16} />
      <input
        class="app-slider"
        type="range"
        min="0"
        max="100"
        step="1"
        value={volumePercent}
        disabled={!volumeEnabled}
        aria-label="Clip volume"
        style:--slider-fill={`${volumePercent}%`}
        on:input={(event) => dispatch('volume', { volume: Number(event.currentTarget.value) / 100 })}
      />
      <span class="mono">{volumePercent}%</span>
    </label>
    <span class="transport-row__divider" aria-hidden="true"></span>
    <div class="transport-row__zoom">
      <input
        class="app-slider"
        type="range"
        min="0"
        max={MAX_ZOOM}
        value={zoom}
        disabled={!hasClip}
        aria-label="Timeline zoom"
        title="Timeline zoom (Ctrl+scroll on the timeline)"
        style:--slider-fill={`${(zoom / MAX_ZOOM) * 100}%`}
        on:input={(event) => dispatch('zoomChange', { zoom: Number(event.currentTarget.value) })}
      />
      <IconButton icon="scaleFit" title="Fit timeline to window" disabled={!hasClip} on:click={() => dispatch('zoomFit')} />
      <IconButton icon="zoomRange" title="Zoom to In → Out (Z)" disabled={!hasClip || !canUseRange} on:click={() => dispatch('zoomRange')} />
    </div>
  </div>
</div>
