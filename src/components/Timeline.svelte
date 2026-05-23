<script lang="ts">
  import { createEventDispatcher, onMount, tick } from 'svelte';
  import IconButton from './IconButton.svelte';
  import TimelineWaveform from './TimelineWaveform.svelte';
  import { clamp, formatTime } from '../lib/format';
  import { placeMenu } from '../lib/placeMenu';
  import { sequenceToSourceTime as mapSequenceToSourceTime, sourceToSequenceTime as mapSourceToSequenceTime } from '../lib/timelineMapping';
  import type { TimelineBookmark } from '../lib/types';
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
  /** Filled by layout; used for waveform canvas sizing. */
  let audioLaneHeight = 58;
  export let audioCodec: string | null = null;
  export let audioChannels: number | null = null;
  export let bookmarks: TimelineBookmark[] = [];
  export let selectedBookmarkId: string | null = null;
  export let waveformPeaks: number[] = [];
  export let waveformLoading = false;
  export let sourceDuration = 0;

  const TRACK_SPLIT_STORAGE_KEY = 'cutdown-track-split';
  const RULER_HEIGHT = 25;
  const RESIZE_BAR_HEIGHT = 6;
  const MIN_VIDEO_TRACK_HEIGHT = 42;
  const MIN_AUDIO_TRACK_HEIGHT = 38;

  let scrollArea: HTMLDivElement;
  let videoScroll: HTMLDivElement;
  let audioScroll: HTMLDivElement;
  let ruler: HTMLDivElement;
  let timelineBody: HTMLDivElement;
  let trackSplitRatio = 0.55;
  let activeSeek = false;
  let activeMarker: 'start' | 'end' | null = null;
  let activeResize = false;
  let resizeStartY = 0;
  let resizeStartSplitRatio = 0.55;
  let contextMenu:
    | {
        x: number;
        y: number;
        seconds: number;
      }
    | null = null;
  let bookmarkMenu: { x: number; y: number; id: string } | null = null;

  type SegmentLayout = {
    segment: TimelineSegment;
    left: number;
    width: number;
  };

  const MAX_ZOOM = 320;

  const dispatch = createEventDispatcher<{
    seek: { seconds: number };
    selectSegment: { id: string | null };
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
    reorderSegment: { id: string; toIndex: number };
    bookmarkClick: { id: string };
    bookmarkRemove: { id: string };
    bookmarkSelect: { id: string };
    bookmarkEdit: { id: string };
  }>();

  let segmentDrag:
    | {
        id: string;
        fromIndex: number;
        startX: number;
        active: boolean;
      }
    | null = null;
  let segmentDragTargetIndex: number | null = null;

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
  $: bookmarkLayouts = bookmarks.map((bookmark) => ({
    bookmark,
    left: sourceToSequenceTime(bookmark.time) * pixelsPerSecond,
  }));

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
    return mapSequenceToSourceTime(segments, sequenceTime);
  }

  function sourceToSequenceTime(sourceTime: number): number {
    return mapSourceToSequenceTime(segments, sourceTime);
  }

  function updateSeek(event: PointerEvent): void {
    if (!activeSeek || disabled || outputDuration <= 0) {
      return;
    }

    dispatch('seek', { seconds: sequenceToSourceTime(xToSequenceSeconds(event.clientX)) });
  }

  function handleRulerKeydown(event: KeyboardEvent): void {
    if (disabled || outputDuration <= 0) {
      return;
    }

    const currentSequenceTime = sourceToSequenceTime(currentTime);
    const step = event.shiftKey ? 5 : 1 / Math.max(1, Math.round(pixelsPerSecond));
    let nextSequenceTime: number | null = null;

    if (event.key === 'ArrowLeft') {
      nextSequenceTime = currentSequenceTime - step;
    } else if (event.key === 'ArrowRight') {
      nextSequenceTime = currentSequenceTime + step;
    } else if (event.key === 'Home') {
      nextSequenceTime = 0;
    } else if (event.key === 'End') {
      nextSequenceTime = outputDuration;
    }

    if (nextSequenceTime === null) {
      return;
    }

    event.preventDefault();
    dispatch('seek', { seconds: sequenceToSourceTime(clamp(nextSequenceTime, 0, outputDuration)) });
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

  function xToSequenceSecondsOnTrack(clientX: number): number {
    if (!videoScroll) {
      return 0;
    }

    const rect = videoScroll.getBoundingClientRect();
    const localX = clientX - rect.left + videoScroll.scrollLeft;
    return clamp(localX / pixelsPerSecond, 0, outputDuration);
  }

  function segmentIndexForSequenceTime(sequenceTime: number): number {
    let cursor = 0;

    for (let index = 0; index < segments.length; index += 1) {
      const segmentLength = Math.max(0, segments[index].sourceEnd - segments[index].sourceStart);
      if (sequenceTime <= cursor + segmentLength / 2) {
        return index;
      }

      cursor += segmentLength;
    }

    return Math.max(0, segments.length - 1);
  }

  function startSegmentDrag(segment: TimelineSegment, index: number, event: PointerEvent): void {
    if (disabled || event.button !== 0) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    closeContextMenu();
    if (selectedSegmentId !== segment.id) {
      dispatch('selectSegment', { id: segment.id });
    }
    segmentDrag = {
      id: segment.id,
      fromIndex: index,
      startX: event.clientX,
      active: false,
    };
    segmentDragTargetIndex = index;
    videoScroll?.setPointerCapture(event.pointerId);
  }

  function updateSegmentDrag(event: PointerEvent): void {
    if (!segmentDrag) {
      return;
    }

    if (!segmentDrag.active && Math.abs(event.clientX - segmentDrag.startX) > 8) {
      segmentDrag = { ...segmentDrag, active: true };
    }

    if (segmentDrag.active) {
      segmentDragTargetIndex = segmentIndexForSequenceTime(xToSequenceSecondsOnTrack(event.clientX));
    }
  }

  function stopSegmentDrag(event: PointerEvent): void {
    if (!segmentDrag) {
      return;
    }

    if (videoScroll?.hasPointerCapture(event.pointerId)) {
      videoScroll.releasePointerCapture(event.pointerId);
    }

    if (
      segmentDrag.active &&
      segmentDragTargetIndex !== null &&
      segmentDragTargetIndex !== segmentDrag.fromIndex
    ) {
      dispatch('reorderSegment', { id: segmentDrag.id, toIndex: segmentDragTargetIndex });
    }

    segmentDrag = null;
    segmentDragTargetIndex = null;
  }

  function handlePointerMove(event: PointerEvent): void {
    updateSeek(event);
    updateMarker(event);
    updateTrackResize(event);
    updateSegmentDrag(event);
  }

  function handlePointerUp(event: PointerEvent): void {
    stopSeek(event);
    stopMarkerDrag(event);
    stopTrackResize(event);
    stopSegmentDrag(event);
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

  function readTrackSplitRatio(): number {
    const raw = localStorage.getItem(TRACK_SPLIT_STORAGE_KEY);
    const parsed = raw ? Number.parseFloat(raw) : Number.NaN;
    return Number.isFinite(parsed) ? clamp(parsed, 0.22, 0.78) : 0.55;
  }

  function persistTrackSplitRatio(): void {
    localStorage.setItem(TRACK_SPLIT_STORAGE_KEY, String(trackSplitRatio));
  }

  function usableTrackHeight(): number {
    if (!timelineBody) {
      return MIN_VIDEO_TRACK_HEIGHT + MIN_AUDIO_TRACK_HEIGHT;
    }

    return Math.max(
      MIN_VIDEO_TRACK_HEIGHT + MIN_AUDIO_TRACK_HEIGHT,
      timelineBody.clientHeight - RULER_HEIGHT - RESIZE_BAR_HEIGHT,
    );
  }

  function syncTrackHeightProps(video: number, audio: number): void {
    if (video === videoTrackHeight && audio === audioTrackHeight) {
      return;
    }

    videoTrackHeight = video;
    audioTrackHeight = audio;
    dispatch('trackHeightChange', { videoHeight: video, audioHeight: audio });
  }

  function measureTrackHeights(): void {
    if (!timelineBody) {
      return;
    }

    const usable = usableTrackHeight();
    const video = clamp(
      Math.round(usable * trackSplitRatio),
      MIN_VIDEO_TRACK_HEIGHT,
      usable - MIN_AUDIO_TRACK_HEIGHT,
    );
    const audio = Math.max(MIN_AUDIO_TRACK_HEIGHT, usable - video);
    syncTrackHeightProps(video, audio);
  }

  $: trackGridTemplateRows = `25px minmax(${MIN_VIDEO_TRACK_HEIGHT}px, ${trackSplitRatio}fr) 6px minmax(${MIN_AUDIO_TRACK_HEIGHT}px, ${1 - trackSplitRatio}fr)`;

  onMount(() => {
    trackSplitRatio = readTrackSplitRatio();

    const bodyObserver = new ResizeObserver(() => {
      if (!activeResize) {
        measureTrackHeights();
      }

      if (audioScroll) {
        audioLaneHeight = audioScroll.clientHeight;
      }
    });

    if (timelineBody) {
      bodyObserver.observe(timelineBody);
    }

    void tick().then(() => {
      measureTrackHeights();

      if (audioScroll) {
        audioLaneHeight = audioScroll.clientHeight;
      }
    });

    return () => bodyObserver.disconnect();
  });

  function startTrackResize(event: PointerEvent): void {
    event.preventDefault();
    activeResize = true;
    resizeStartY = event.clientY;
    resizeStartSplitRatio = trackSplitRatio;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function updateTrackResize(event: PointerEvent): void {
    if (!activeResize) {
      return;
    }

    const usable = Math.max(usableTrackHeight(), 1);
    const delta = event.clientY - resizeStartY;
    trackSplitRatio = clamp(resizeStartSplitRatio + delta / usable, 0.22, 0.78);
  }

  function stopTrackResize(event: PointerEvent): void {
    const target = event.currentTarget as HTMLElement | null;
    if (target?.hasPointerCapture?.(event.pointerId)) {
      target.releasePointerCapture(event.pointerId);
    }

    if (activeResize) {
      persistTrackSplitRatio();
    }

    activeResize = false;
    measureTrackHeights();
  }

  function toggleAudioExpanded(): void {
    trackSplitRatio = trackSplitRatio < 0.45 ? 0.58 : 0.38;
    persistTrackSplitRatio();
    measureTrackHeights();
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

  function toggleSegmentSelection(segmentId: string): void {
    dispatch('selectSegment', { id: selectedSegmentId === segmentId ? null : segmentId });
  }

  function selectSegment(segment: TimelineSegment, event: PointerEvent): void {
    event.preventDefault();
    event.stopPropagation();
    activeSeek = false;
    closeContextMenu();
    toggleSegmentSelection(segment.id);
  }

  function deselectSegment(): void {
    if (selectedSegmentId) {
      dispatch('selectSegment', { id: null });
    }
  }

  function onTrackBackgroundPointerDown(event: PointerEvent): void {
    if (disabled || event.button !== 0) {
      return;
    }

    if (event.target !== event.currentTarget) {
      return;
    }

    closeContextMenu();
    deselectSegment();
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
    bookmarkMenu = null;
  }

  function bookmarkLabel(bookmark: TimelineBookmark): string {
    return bookmark.label?.trim() || formatTime(bookmark.time);
  }

  function openBookmarkMenu(id: string, event: MouseEvent): void {
    event.preventDefault();
    event.stopPropagation();
    closeContextMenu();
    bookmarkMenu = { x: event.clientX, y: event.clientY, id };
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

  <div
    class="timeline__body"
    bind:this={timelineBody}
    style={`grid-template-rows: ${trackGridTemplateRows}`}
  >
    {#if disabled}
      <div class="timeline__empty">Open a video or drop a file to start cutting.</div>
    {/if}
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
          on:keydown={handleRulerKeydown}
        >
          {#each ticks as tick}
            <div class:major={tick.major} class="timeline__ruler-tick" style={`left: ${tick.left}px`}></div>
            {#if tick.major}
              <span style={`left: ${tick.left}px`}>{formatTime(tick.seconds)}</span>
            {/if}
          {/each}

          {#each bookmarkLayouts as layout (layout.bookmark.id)}
            <button
              type="button"
              class="timeline__bookmark"
              class:selected={layout.bookmark.id === selectedBookmarkId}
              style={`left: ${layout.left}px`}
              title={`${bookmarkLabel(layout.bookmark)} · ${formatTime(layout.bookmark.time)}`}
              aria-label={`Marker ${bookmarkLabel(layout.bookmark)} at ${formatTime(layout.bookmark.time)}`}
              on:pointerdown|stopPropagation={(event) => {
                event.preventDefault();
                dispatch('bookmarkSelect', { id: layout.bookmark.id });
                dispatch('bookmarkClick', { id: layout.bookmark.id });
              }}
              on:dblclick|stopPropagation|preventDefault={() =>
                dispatch('bookmarkEdit', { id: layout.bookmark.id })}
              on:contextmenu|stopPropagation|preventDefault={(event) =>
                openBookmarkMenu(layout.bookmark.id, event)}
            ></button>
            <span
              class="timeline__bookmark-label"
              class:selected={layout.bookmark.id === selectedBookmarkId}
              style={`left: ${layout.left}px`}
            >
              {bookmarkLabel(layout.bookmark)}
            </span>
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

    <div class="timeline__track-head timeline__track-head--video">
      <strong>Video</strong>
      <span>{segments.length} segment{segments.length === 1 ? '' : 's'}</span>
    </div>
    <div
      bind:this={videoScroll}
      class="timeline__scroll timeline__scroll--track"
      role="region"
      aria-label="Video track"
      on:contextmenu={openContextMenu}
    >
      <div
        class="timeline__content timeline__content--track"
        style={`width: ${timelineWidth}px`}
        on:pointerdown={onTrackBackgroundPointerDown}
      >
        {#each ticks as tick}
          <div class="timeline__tick" style={`left: ${tick.left}px`}></div>
        {/each}
        {#if normalizedRange}
          <div class="timeline__track-range" style={`left: ${rangeLeft}px; width: ${rangeWidth}px`}></div>
        {/if}
        {#each segmentLayouts as layout, index (layout.segment.id)}
          <button
            aria-label={`Video segment from ${formatTime(layout.segment.sourceStart)} to ${formatTime(layout.segment.sourceEnd)}`}
            class:selected={layout.segment.id === selectedSegmentId}
            class:timeline__segment--sole={segments.length === 1}
            class:timeline__segment--drag-target={segmentDrag?.active && segmentDragTargetIndex === index}
            class:timeline__segment--dragging={segmentDrag?.active && segmentDrag.id === layout.segment.id}
            class="timeline__segment timeline__segment--video"
            style={segmentStyle(layout)}
            type="button"
            on:pointerdown={(event) => startSegmentDrag(layout.segment, index, event)}
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

    <button
      type="button"
      class="timeline__resize"
      style="grid-column: 1 / -1"
      aria-label="Resize tracks"
      on:pointerdown={startTrackResize}
    ></button>

    <button
      type="button"
      class="timeline__track-head timeline__track-head--audio"
      on:dblclick={toggleAudioExpanded}
    >
      <strong>Audio</strong>
      <span>
        {audioLabel}{waveformLoading ? ' · loading waveform…' : waveformPeaks.length ? ' · waveform' : ''}
      </span>
    </button>
    <div
      bind:this={audioScroll}
      class="timeline__scroll timeline__scroll--track"
      role="region"
      aria-label="Audio track"
      on:contextmenu={openContextMenu}
    >
      <div
        class="timeline__content timeline__content--track timeline__content--audio"
        style={`width: ${timelineWidth}px`}
        on:pointerdown={onTrackBackgroundPointerDown}
      >
        {#if waveformPeaks.length > 0 && audioCodec}
          <TimelineWaveform
            peaks={waveformPeaks}
            {sourceDuration}
            {segments}
            {selectedSegmentId}
            pixelsPerSecond={pixelsPerSecond}
            width={timelineWidth}
            height={Math.max(24, audioLaneHeight - 8)}
            outputDuration={outputDuration}
          />
        {/if}
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

  {#if bookmarkMenu}
    {@const bookmarkMenuId = bookmarkMenu.id}
    <div
      class="timeline-menu"
      role="menu"
      style={`left: ${bookmarkMenu.x}px; top: ${bookmarkMenu.y}px`}
      use:placeMenu={{ x: bookmarkMenu.x, y: bookmarkMenu.y }}
    >
      <button type="button" on:click={() => {
        dispatch('bookmarkEdit', { id: bookmarkMenuId });
        closeContextMenu();
      }}>Edit label</button>
      <button type="button" on:click={() => {
        dispatch('bookmarkRemove', { id: bookmarkMenuId });
        closeContextMenu();
      }}>Delete marker</button>
    </div>
  {/if}

  {#if contextMenu}
    <div
      class="timeline-menu"
      role="menu"
      style={`left: ${contextMenu.x}px; top: ${contextMenu.y}px`}
      use:placeMenu={{ x: contextMenu.x, y: contextMenu.y }}
    >
      <button type="button" on:click={() => runContextAction('split')}>Split here</button>
      <button type="button" disabled={!selectedSegmentId} on:click={() => runContextAction('delete')}>Delete selected segment</button>
      <button type="button" disabled={!selectedSegmentId} on:click={() => {
        deselectSegment();
        closeContextMenu();
      }}>Deselect segment</button>
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
