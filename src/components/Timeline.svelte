<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  import IconButton from './IconButton.svelte';
  import { clamp, formatTime } from '../lib/format';
  import type { TimelineSegment } from '../stores/editor';

  export let duration = 0;
  export let currentTime = 0;
  export let segments: TimelineSegment[] = [];
  export let selectedSegmentId: string | null = null;
  export let disabled = true;
  export let rangeStart: number | null = null;
  export let rangeEnd: number | null = null;
  export let zoom = 28;
  export let videoTrackHeight = 68;
  export let audioTrackHeight = 58;
  export let audioCodec: string | null = null;
  export let audioChannels: number | null = null;

  let scrollArea: HTMLDivElement;
  let videoScroll: HTMLDivElement;
  let audioScroll: HTMLDivElement;
  let ruler: HTMLDivElement;
  let activeSeek = false;
  let activeMarker: 'start' | 'end' | null = null;
  let activeResize = false;
  let resizeStartY = 0;
  let resizeStartVideoHeight = 0;
  let resizeStartAudioHeight = 0;
  let contextMenu:
    | {
        x: number;
        y: number;
        seconds: number;
      }
    | null = null;

  type SegmentLayout = {
    segment: TimelineSegment;
    left: number;
    width: number;
  };

  const MAX_ZOOM = 320;

  const dispatch = createEventDispatcher<{
    seek: { seconds: number };
    selectSegment: { id: string };
    rangeChange: { start: number; end: number };
    zoomChange: { zoom: number };
    trackHeightChange: { videoHeight: number; audioHeight: number };
    splitAt: { seconds: number };
    deleteSelected: void;
    zoomFit: void;
    zoomRange: void;
    clearRange: void;
    splitRange: void;
    keepRange: void;
    trimOutsideRange: void;
    toggleRangeLoop: void;
  }>();

  function zoomLevelForSequenceSeconds(sequenceSeconds: number): number {
    if (sequenceSeconds <= 0) {
      return 28;
    }

    const viewport = scrollArea?.clientWidth ?? 640;
    return clamp((viewport / sequenceSeconds - 1.5) / 0.42, 0, MAX_ZOOM);
  }

  export function zoomToFitView(): void {
    if (disabled || outputDuration <= 0) {
      return;
    }

    const nextZoom = zoomLevelForSequenceSeconds(outputDuration);
    dispatch('zoomChange', { zoom: nextZoom });

    if (scrollArea) {
      scrollArea.scrollLeft = 0;
    }
  }

  export async function zoomToSourceRange(start: number, end: number): Promise<void> {
    if (disabled || outputDuration <= 0) {
      return;
    }

    const seqStart = sourceToSequenceTime(start);
    const seqEnd = sourceToSequenceTime(end);
    const rangeSeconds = Math.max(0.05, seqEnd - seqStart);
    const nextZoom = zoomLevelForSequenceSeconds(rangeSeconds);

    dispatch('zoomChange', { zoom: nextZoom });
    await tick();

    if (scrollArea) {
      scrollArea.scrollLeft = Math.max(0, seqStart * pixelsPerSecond - 24);
      syncTrackScroll();
    }
  }

  $: outputDuration = segments.reduce((total, segment) => total + Math.max(0, segment.sourceEnd - segment.sourceStart), 0);
  $: pixelsPerSecond = 1.5 + zoom * 0.42;
  $: timelineWidth = Math.max(640, outputDuration * pixelsPerSecond);
  $: playheadLeft = sourceToSequenceTime(currentTime) * pixelsPerSecond;
  $: normalizedRange =
    rangeStart !== null && rangeEnd !== null
      ? {
          start: Math.min(rangeStart, rangeEnd),
          end: Math.max(rangeStart, rangeEnd),
        }
      : null;
  $: rangeSequenceStart = normalizedRange ? sourceToSequenceTime(normalizedRange.start) : 0;
  $: rangeSequenceEnd = normalizedRange ? sourceToSequenceTime(normalizedRange.end) : 0;
  $: rangeLeft = rangeSequenceStart * pixelsPerSecond;
  $: rangeWidth = Math.max(0, (rangeSequenceEnd - rangeSequenceStart) * pixelsPerSecond);
  $: segmentLayouts = buildSegmentLayouts(segments, pixelsPerSecond);
  $: ticks = buildTicks(outputDuration, pixelsPerSecond);
  $: audioLabel = audioCodec ? `${audioCodec}${audioChannels ? ` ${audioChannels}ch` : ''}` : 'no audio';

  function buildTicks(totalSeconds: number, pxPerSecond: number): Array<{ id: number; seconds: number; left: number; major: boolean }> {
    if (totalSeconds <= 0) {
      return [];
    }

    const targetSpacing = 90;
    const candidates = [1, 2, 5, 10, 15, 30, 60, 120, 300, 600];
    const step = candidates.find((candidate) => candidate * pxPerSecond >= targetSpacing) ?? candidates[candidates.length - 1];
    const ticks = [];

    for (let seconds = 0; seconds <= totalSeconds + 0.001; seconds += step) {
      ticks.push({
        id: ticks.length,
        seconds,
        left: seconds * pxPerSecond,
        major: ticks.length % 2 === 0,
      });
    }

    return ticks;
  }

  function xToSequenceSeconds(clientX: number): number {
    const rect = scrollArea.getBoundingClientRect();
    const localX = clientX - rect.left + scrollArea.scrollLeft;
    return clamp(localX / pixelsPerSecond, 0, outputDuration);
  }

  function sequenceToX(sequenceTime: number): number {
    return clamp(sequenceTime, 0, outputDuration) * pixelsPerSecond;
  }

  function sequenceToSourceTime(sequenceTime: number): number {
    let cursor = 0;

    for (const segment of segments) {
      const segmentLength = Math.max(0, segment.sourceEnd - segment.sourceStart);

      if (sequenceTime <= cursor + segmentLength) {
        return segment.sourceStart + clamp(sequenceTime - cursor, 0, segmentLength);
      }

      cursor += segmentLength;
    }

    return segments[segments.length - 1]?.sourceEnd ?? 0;
  }

  function sourceToSequenceTime(sourceTime: number): number {
    let cursor = 0;
    let nearestSequenceTime = 0;
    let nearestDistance = Number.POSITIVE_INFINITY;

    for (const segment of segments) {
      const segmentLength = Math.max(0, segment.sourceEnd - segment.sourceStart);

      if (sourceTime >= segment.sourceStart && sourceTime <= segment.sourceEnd) {
        return cursor + clamp(sourceTime - segment.sourceStart, 0, segmentLength);
      }

      const startDistance = Math.abs(sourceTime - segment.sourceStart);
      if (startDistance < nearestDistance) {
        nearestDistance = startDistance;
        nearestSequenceTime = cursor;
      }

      const endDistance = Math.abs(sourceTime - segment.sourceEnd);
      if (endDistance < nearestDistance) {
        nearestDistance = endDistance;
        nearestSequenceTime = cursor + segmentLength;
      }

      cursor += segmentLength;
    }

    return clamp(nearestSequenceTime, 0, outputDuration);
  }

  function updateSeek(event: PointerEvent): void {
    if (!activeSeek || disabled || outputDuration <= 0) {
      return;
    }

    dispatch('seek', { seconds: sequenceToSourceTime(xToSequenceSeconds(event.clientX)) });
  }

  function startSeek(event: PointerEvent): void {
    if (disabled || outputDuration <= 0) {
      return;
    }

    event.preventDefault();
    closeContextMenu();
    activeSeek = true;
    ruler.setPointerCapture(event.pointerId);
    updateSeek(event);
  }

  function stopSeek(event: PointerEvent): void {
    if (ruler?.hasPointerCapture(event.pointerId)) {
      ruler.releasePointerCapture(event.pointerId);
    }

    activeSeek = false;
  }

  function startMarkerDrag(marker: 'start' | 'end', event: PointerEvent): void {
    if (disabled || outputDuration <= 0 || !normalizedRange) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    closeContextMenu();
    activeMarker = marker;
    ruler.setPointerCapture(event.pointerId);
    updateMarker(event);
  }

  function updateMarker(event: PointerEvent): void {
    if (!activeMarker || !normalizedRange) {
      return;
    }

    const sourceTime = sequenceToSourceTime(xToSequenceSeconds(event.clientX));
    const nextStart = activeMarker === 'start' ? sourceTime : normalizedRange.start;
    const nextEnd = activeMarker === 'end' ? sourceTime : normalizedRange.end;
    dispatch('rangeChange', {
      start: clamp(nextStart, 0, duration),
      end: clamp(nextEnd, 0, duration),
    });
  }

  function stopMarkerDrag(event: PointerEvent): void {
    if (ruler?.hasPointerCapture(event.pointerId)) {
      ruler.releasePointerCapture(event.pointerId);
    }

    activeMarker = null;
  }

  function handlePointerMove(event: PointerEvent): void {
    updateSeek(event);
    updateMarker(event);
    updateTrackResize(event);
  }

  function handlePointerUp(event: PointerEvent): void {
    stopSeek(event);
    stopMarkerDrag(event);
    stopTrackResize(event);
  }

  function updateZoom(nextZoom: number): void {
    dispatch('zoomChange', { zoom: clamp(nextZoom, 0, MAX_ZOOM) });
  }

  function handleZoomInput(event: Event): void {
    updateZoom(Number((event.currentTarget as HTMLInputElement).value));
  }

  function handleWheel(event: WheelEvent): void {
    if (!event.ctrlKey || disabled) {
      return;
    }

    event.preventDefault();
    updateZoom(zoom + (event.deltaY < 0 ? 6 : -6));
  }

  function syncTrackScroll(): void {
    const nextScrollLeft = scrollArea?.scrollLeft ?? 0;

    if (videoScroll) {
      videoScroll.scrollLeft = nextScrollLeft;
    }

    if (audioScroll) {
      audioScroll.scrollLeft = nextScrollLeft;
    }
  }

  function startTrackResize(event: PointerEvent): void {
    event.preventDefault();
    activeResize = true;
    resizeStartY = event.clientY;
    resizeStartVideoHeight = videoTrackHeight;
    resizeStartAudioHeight = audioTrackHeight;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function updateTrackResize(event: PointerEvent): void {
    if (!activeResize) {
      return;
    }

    const delta = event.clientY - resizeStartY;
    dispatch('trackHeightChange', {
      videoHeight: clamp(resizeStartVideoHeight + delta, 42, 150),
      audioHeight: clamp(resizeStartAudioHeight - delta, 38, 150),
    });
  }

  function stopTrackResize(event: PointerEvent): void {
    const target = event.currentTarget as HTMLElement | null;
    if (target?.hasPointerCapture?.(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }

    activeResize = false;
  }

  function toggleAudioExpanded(): void {
    dispatch('trackHeightChange', {
      videoHeight: videoTrackHeight,
      audioHeight: audioTrackHeight < 92 ? 116 : 58,
    });
  }

  function segmentSequenceStart(index: number): number {
    return segments
      .slice(0, index)
      .reduce((total, segment) => total + Math.max(0, segment.sourceEnd - segment.sourceStart), 0);
  }

  function buildSegmentLayouts(sourceSegments: TimelineSegment[], pxPerSecond: number): SegmentLayout[] {
    let cursor = 0;

    return sourceSegments.map((segment) => {
      const segmentLength = Math.max(0, segment.sourceEnd - segment.sourceStart);
      const layout = {
        segment,
        left: cursor * pxPerSecond,
        width: Math.max(12, segmentLength * pxPerSecond),
      };
      cursor += segmentLength;
      return layout;
    });
  }

  function segmentStyle(layout: SegmentLayout): string {
    return `left: ${layout.left}px; width: ${layout.width}px`;
  }

  function selectSegment(segment: TimelineSegment, event: PointerEvent): void {
    event.preventDefault();
    event.stopPropagation();
    activeSeek = false;
    closeContextMenu();
    dispatch('selectSegment', { id: segment.id });
  }

  function openContextMenu(event: MouseEvent): void {
    if (disabled || outputDuration <= 0) {
      return;
    }

    event.preventDefault();
    contextMenu = {
      x: event.clientX,
      y: event.clientY,
      seconds: sequenceToSourceTime(xToSequenceSeconds(event.clientX)),
    };
  }

  function closeContextMenu(): void {
    contextMenu = null;
  }

  function runContextAction(
    action:
      | 'split'
      | 'delete'
      | 'fit'
      | 'zoomRange'
      | 'splitRange'
      | 'keepRange'
      | 'trimOutside'
      | 'loopRange'
      | 'clear',
  ): void {
    if (!contextMenu && action === 'split') {
      return;
    }

    if (action === 'split' && contextMenu) {
      dispatch('splitAt', { seconds: contextMenu.seconds });
    } else if (action === 'delete') {
      dispatch('deleteSelected');
    } else if (action === 'fit') {
      dispatch('zoomFit');
    } else if (action === 'zoomRange') {
      dispatch('zoomRange');
    } else if (action === 'splitRange') {
      dispatch('splitRange');
    } else if (action === 'keepRange') {
      dispatch('keepRange');
    } else if (action === 'trimOutside') {
      dispatch('trimOutsideRange');
    } else if (action === 'loopRange') {
      dispatch('toggleRangeLoop');
    } else if (action === 'clear') {
      dispatch('clearRange');
    }

    closeContextMenu();
  }
</script>

<svelte:window on:pointermove={handlePointerMove} on:pointerup={handlePointerUp} on:pointercancel={handlePointerUp} on:click={closeContextMenu} />

<section class="timeline" aria-label="Vegas-style timeline" on:contextmenu={openContextMenu} on:wheel={handleWheel}>
  <header class="timeline__controls">
    <div class="timeline__readouts">
      <span>Now <strong>{formatTime(currentTime)}</strong></span>
      <span>Out <strong>{formatTime(outputDuration)}</strong></span>
      <span>Range <strong>{normalizedRange ? `${formatTime(normalizedRange.start)} - ${formatTime(normalizedRange.end)}` : 'none'}</strong></span>
    </div>
    <div class="timeline__zoom">
      <span>Zoom</span>
      <input
        class="app-slider"
        type="range"
        min="0"
        max={MAX_ZOOM}
        value={zoom}
        disabled={disabled}
        aria-label="Timeline zoom"
        style={`--slider-fill: ${(zoom / MAX_ZOOM) * 100}%`}
        on:input={handleZoomInput}
      />
      <IconButton icon="scaleFit" title="Fit timeline to window" variant="mini" disabled={disabled} on:click={() => dispatch('zoomFit')} />
      <IconButton
        icon="zoomRange"
        title="Zoom to I/O range"
        variant="mini"
        disabled={disabled || !normalizedRange}
        on:click={() => dispatch('zoomRange')}
      />
    </div>
  </header>

  <div class="timeline__body">
    <div class="timeline__track-head timeline__track-head--spacer">Tracks</div>
    <div class="timeline__scroll" bind:this={scrollArea} on:scroll={syncTrackScroll}>
      <div class="timeline__content" style={`width: ${timelineWidth}px`}>
        <div
          bind:this={ruler}
          class="timeline__ruler"
          class:is-disabled={disabled}
          role="slider"
          aria-label="Seek"
          aria-valuemin="0"
          aria-valuemax={outputDuration}
          aria-valuenow={sourceToSequenceTime(currentTime)}
          tabindex={disabled ? -1 : 0}
          on:pointerdown={startSeek}
        >
          {#each ticks as tick}
            <div class:major={tick.major} class="timeline__ruler-tick" style={`left: ${tick.left}px`}></div>
            {#if tick.major}
              <span style={`left: ${tick.left}px`}>{formatTime(tick.seconds)}</span>
            {/if}
          {/each}

          {#if normalizedRange}
            <div class="timeline__range-fill" style={`left: ${rangeLeft}px; width: ${rangeWidth}px`}></div>
            <button
              type="button"
              class="timeline__marker timeline__marker--in"
              style={`left: ${rangeLeft}px`}
              aria-label="Range start"
              on:pointerdown={(event) => startMarkerDrag('start', event)}
            ></button>
            <button
              type="button"
              class="timeline__marker timeline__marker--out"
              style={`left: ${rangeLeft + rangeWidth}px`}
              aria-label="Range end"
              on:pointerdown={(event) => startMarkerDrag('end', event)}
            ></button>
          {/if}

          <div class="timeline__playhead timeline__playhead--ruler" style={`left: ${playheadLeft}px`}></div>
        </div>
      </div>
    </div>

    <div class="timeline__track-head" style={`height: ${videoTrackHeight}px`}>
      <strong>Video</strong>
      <span>{segments.length} segment{segments.length === 1 ? '' : 's'}</span>
    </div>
    <div bind:this={videoScroll} class="timeline__scroll timeline__scroll--track" role="region" aria-label="Video track" on:contextmenu={openContextMenu}>
      <div class="timeline__content timeline__content--track" style={`width: ${timelineWidth}px; height: ${videoTrackHeight}px`}>
        {#each ticks as tick}
          <div class="timeline__tick" style={`left: ${tick.left}px`}></div>
        {/each}
        {#if normalizedRange}
          <div class="timeline__track-range" style={`left: ${rangeLeft}px; width: ${rangeWidth}px`}></div>
        {/if}
        {#each segmentLayouts as layout (layout.segment.id)}
          <button
            aria-label={`Video segment from ${formatTime(layout.segment.sourceStart)} to ${formatTime(layout.segment.sourceEnd)}`}
            class:selected={layout.segment.id === selectedSegmentId}
            class="timeline__segment timeline__segment--video"
            style={segmentStyle(layout)}
            type="button"
            on:pointerdown={(event) => selectSegment(layout.segment, event)}
            on:dblclick={(event) => {
              event.stopPropagation();
              dispatch('seek', { seconds: layout.segment.sourceStart });
            }}
          >
            <span>{formatTime(layout.segment.sourceStart)}</span>
            <strong>{formatTime(layout.segment.sourceEnd - layout.segment.sourceStart)}</strong>
          </button>
        {/each}
        <div class="timeline__playhead" style={`left: ${playheadLeft}px`}></div>
      </div>
    </div>

    <button type="button" class="timeline__resize" aria-label="Resize tracks" on:pointerdown={startTrackResize}></button>
    <div class="timeline__resize-fill"></div>

    <button
      type="button"
      class="timeline__track-head timeline__track-head--audio"
      style={`height: ${audioTrackHeight}px`}
      on:dblclick={toggleAudioExpanded}
    >
      <strong>Audio</strong>
      <span>{audioLabel}</span>
    </button>
    <div bind:this={audioScroll} class="timeline__scroll timeline__scroll--track" role="region" aria-label="Audio track" on:contextmenu={openContextMenu}>
      <div class="timeline__content timeline__content--track" style={`width: ${timelineWidth}px; height: ${audioTrackHeight}px`}>
        {#each ticks as tick}
          <div class="timeline__tick" style={`left: ${tick.left}px`}></div>
        {/each}
        {#if normalizedRange}
          <div class="timeline__track-range" style={`left: ${rangeLeft}px; width: ${rangeWidth}px`}></div>
        {/if}
        {#each segmentLayouts as layout (layout.segment.id)}
          <button
            aria-label={`Audio segment from ${formatTime(layout.segment.sourceStart)} to ${formatTime(layout.segment.sourceEnd)}`}
            class:selected={layout.segment.id === selectedSegmentId}
            class="timeline__segment timeline__segment--audio"
            style={segmentStyle(layout)}
            type="button"
            on:pointerdown={(event) => selectSegment(layout.segment, event)}
          >
            <span>{audioLabel}</span>
          </button>
        {/each}
        <div class="timeline__playhead" style={`left: ${playheadLeft}px`}></div>
      </div>
    </div>
  </div>

  {#if contextMenu}
    <div class="timeline-menu" role="menu" style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px`}>
      <button type="button" on:click={() => runContextAction('split')}>Split here</button>
      <button type="button" disabled={!selectedSegmentId} on:click={() => runContextAction('delete')}>Delete selected segment</button>
      <button type="button" on:click={() => runContextAction('fit')}>Zoom to fit</button>
      <button type="button" disabled={!normalizedRange} on:click={() => runContextAction('zoomRange')}>Zoom to range</button>
      <button type="button" disabled={!normalizedRange} on:click={() => runContextAction('splitRange')}>Split at I and O</button>
      <button type="button" disabled={!normalizedRange} on:click={() => runContextAction('keepRange')}>Keep only range</button>
      <button type="button" disabled={!normalizedRange} on:click={() => runContextAction('trimOutside')}>Trim outside range</button>
      <button type="button" disabled={!normalizedRange} on:click={() => runContextAction('loopRange')}>Loop preview range</button>
      <button type="button" disabled={!normalizedRange} on:click={() => runContextAction('clear')}>Clear selection</button>
    </div>
  {/if}
</section>
