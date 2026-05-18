<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { clamp, formatTime } from '../lib/format';
  import type { TimelineSegment } from '../stores/editor';

  export let duration = 0;
  export let currentTime = 0;
  export let segments: TimelineSegment[] = [];
  export let selectedSegmentId: string | null = null;
  export let disabled = true;

  let track: HTMLDivElement;
  let scrubber: HTMLDivElement;
  let activeSeek = false;
  let dragSegmentId: string | null = null;
  let dragStartX = 0;
  let isReordering = false;

  const dispatch = createEventDispatcher<{
    seek: { seconds: number };
    selectSegment: { id: string };
    reorderSegment: { id: string; targetIndex: number };
  }>();

  $: safeDuration = Math.max(duration, 0);
  $: outputDuration = segments.reduce((total, segment) => total + Math.max(0, segment.sourceEnd - segment.sourceStart), 0);
  $: playheadPercent = outputDuration > 0 ? (sourceToSequenceTime(currentTime) / outputDuration) * 100 : 0;
  $: ticks = Array.from({ length: 6 }, (_, index) => ({
    id: index,
    seconds: outputDuration * (index / 5),
    percent: index * 20,
  }));

  function sequenceSecondsFromPointer(event: PointerEvent): number {
    const rect = scrubber.getBoundingClientRect();
    const ratio = clamp((event.clientX - rect.left) / rect.width, 0, 1);
    return ratio * outputDuration;
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

    for (const segment of segments) {
      const segmentLength = Math.max(0, segment.sourceEnd - segment.sourceStart);

      if (sourceTime >= segment.sourceStart && sourceTime <= segment.sourceEnd) {
        return cursor + clamp(sourceTime - segment.sourceStart, 0, segmentLength);
      }

      if (sourceTime > segment.sourceEnd) {
        cursor += segmentLength;
      }
    }

    return clamp(cursor, 0, outputDuration);
  }

  function updateSeek(event: PointerEvent): void {
    if (!activeSeek || disabled || outputDuration <= 0) {
      return;
    }

    dispatch('seek', { seconds: sequenceToSourceTime(clamp(sequenceSecondsFromPointer(event), 0, outputDuration)) });
  }

  function startSeek(event: PointerEvent): void {
    if (disabled || outputDuration <= 0) {
      return;
    }

    event.preventDefault();
    activeSeek = true;
    scrubber.setPointerCapture(event.pointerId);
    updateSeek(event);
  }

  function stopSeek(event: PointerEvent): void {
    if (scrubber?.hasPointerCapture(event.pointerId)) {
      scrubber.releasePointerCapture(event.pointerId);
    }

    activeSeek = false;
  }

  function segmentSequenceStart(index: number): number {
    return segments
      .slice(0, index)
      .reduce((total, segment) => total + Math.max(0, segment.sourceEnd - segment.sourceStart), 0);
  }

  function segmentStyle(segment: TimelineSegment, index: number): string {
    const segmentLength = Math.max(0, segment.sourceEnd - segment.sourceStart);
    const left = outputDuration > 0 ? (segmentSequenceStart(index) / outputDuration) * 100 : 0;
    const width = outputDuration > 0 ? (segmentLength / outputDuration) * 100 : 0;
    return `left: ${left}%; width: ${Math.max(width, 0)}%`;
  }

  function selectSegment(segment: TimelineSegment, event: PointerEvent): void {
    event.preventDefault();
    event.stopPropagation();
    activeSeek = false;
    dragSegmentId = segment.id;
    dragStartX = event.clientX;
    isReordering = false;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    dispatch('selectSegment', { id: segment.id });
  }

  function moveSegment(event: PointerEvent): void {
    if (!dragSegmentId) {
      return;
    }

    if (Math.abs(event.clientX - dragStartX) > 8) {
      isReordering = true;
    }
  }

  function finishSegmentDrag(event: PointerEvent): void {
    if (!dragSegmentId) {
      return;
    }

    if ((event.currentTarget as HTMLElement).hasPointerCapture(event.pointerId)) {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    }

    if (isReordering && segments.length > 1) {
      const rect = track.getBoundingClientRect();
      const ratio = clamp((event.clientX - rect.left) / rect.width, 0, 1);
      const targetIndex = clamp(Math.floor(ratio * segments.length), 0, segments.length - 1);
      dispatch('reorderSegment', { id: dragSegmentId, targetIndex });
    }

    dragSegmentId = null;
    isReordering = false;
  }
</script>

<section class="timeline" aria-label="Trim timeline">
  <div
    bind:this={scrubber}
    class="timeline__ruler"
    class:is-disabled={disabled}
    role="slider"
    aria-label="Seek"
    aria-valuemin="0"
    aria-valuemax={outputDuration}
    aria-valuenow={sourceToSequenceTime(currentTime)}
    tabindex={disabled ? -1 : 0}
    on:pointerdown={startSeek}
    on:pointermove={updateSeek}
    on:pointerup={stopSeek}
    on:pointercancel={stopSeek}
  >
    {#each ticks as tick}
      <span style={`left: ${tick.percent}%`}>{formatTime(tick.seconds)}</span>
    {/each}
  </div>

  <div
    bind:this={track}
    class="timeline__track"
    class:is-disabled={disabled}
  >
    {#each ticks as tick}
      <div class="timeline__tick" style={`left: ${tick.percent}%`}></div>
    {/each}

    {#each segments as segment, index}
      <button
        aria-label={`Segment from ${formatTime(segment.sourceStart)} to ${formatTime(segment.sourceEnd)}`}
        class:dragging={segment.id === dragSegmentId && isReordering}
        class:selected={segment.id === selectedSegmentId}
        class="timeline__segment"
        style={segmentStyle(segment, index)}
        type="button"
        on:pointerdown={(event) => selectSegment(segment, event)}
        on:pointermove={moveSegment}
        on:pointerup={finishSegmentDrag}
        on:pointercancel={finishSegmentDrag}
        on:dblclick={(event) => {
          event.stopPropagation();
          dispatch('seek', { seconds: segment.sourceStart });
        }}
      >
        <span>{formatTime(segment.sourceStart)}</span>
        <strong>{formatTime(segment.sourceEnd - segment.sourceStart)}</strong>
      </button>
    {/each}

    <div class="timeline__playhead" style={`left: ${playheadPercent}%`}></div>
  </div>

  <div class="timeline__meta">
    <span>{segments.length} segment{segments.length === 1 ? '' : 's'}</span>
    <span>Now {formatTime(currentTime)}</span>
    <span>Output {formatTime(outputDuration)}</span>
  </div>
</section>
