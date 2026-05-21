<script lang="ts">
  import { onMount } from 'svelte';
  import type { TimelineSegment } from '../stores/editor';

  export let peaks: number[] = [];
  export let sourceDuration = 0;
  export let segments: TimelineSegment[] = [];
  export let pixelsPerSecond = 1;
  export let width = 0;
  export let height = 0;
  export let outputDuration = 0;
  export let selectedSegmentId: string | null = null;

  let canvas: HTMLCanvasElement | undefined;

  function sequenceRangeForSegment(segmentId: string): { start: number; end: number } | null {
    let cursor = 0;

    for (const segment of segments) {
      const segmentLength = Math.max(0, segment.sourceEnd - segment.sourceStart);
      if (segment.id === segmentId) {
        return { start: cursor, end: cursor + segmentLength };
      }
      cursor += segmentLength;
    }

    return null;
  }

  function sequenceToSourceTime(sequenceTime: number): number {
    let cursor = 0;

    for (const segment of segments) {
      const segmentLength = Math.max(0, segment.sourceEnd - segment.sourceStart);

      if (sequenceTime >= cursor && sequenceTime <= cursor + segmentLength) {
        return segment.sourceStart + Math.max(0, Math.min(sequenceTime - cursor, segmentLength));
      }

      cursor += segmentLength;
    }

    return segments[segments.length - 1]?.sourceEnd ?? 0;
  }

  function peakAtSourceTime(sourceTime: number): number {
    if (!peaks.length || sourceDuration <= 0) {
      return 0;
    }

    const index = Math.floor((sourceTime / sourceDuration) * (peaks.length - 1));
    return peaks[Math.max(0, Math.min(peaks.length - 1, index))] ?? 0;
  }

  function draw(): void {
    if (!canvas || width <= 0 || height <= 0 || !peaks.length || outputDuration <= 0) {
      return;
    }

    const context = canvas.getContext('2d');
    if (!context) {
      return;
    }

    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.max(1, Math.floor(width * dpr));
    canvas.height = Math.max(1, Math.floor(height * dpr));
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    context.setTransform(dpr, 0, 0, dpr, 0, 0);
    context.clearRect(0, 0, width, height);

    const mid = height / 2;
    const selectedRange = selectedSegmentId ? sequenceRangeForSegment(selectedSegmentId) : null;

    for (let x = 0; x < width; x += 1) {
      const sequenceTime = x / pixelsPerSecond;
      const sourceTime = sequenceToSourceTime(sequenceTime);
      const peak = peakAtSourceTime(sourceTime);
      const barHeight = Math.max(1, peak * (height * 0.42));
      const inSelected =
        selectedRange !== null &&
        sequenceTime >= selectedRange.start &&
        sequenceTime <= selectedRange.end;
      context.fillStyle = inSelected ? 'rgba(142, 196, 255, 0.82)' : 'rgba(95, 137, 176, 0.55)';
      context.fillRect(x, mid - barHeight, 1, barHeight * 2);
    }
  }

  $: peaks,
    sourceDuration,
    segments,
    selectedSegmentId,
    pixelsPerSecond,
    width,
    height,
    outputDuration,
    draw();

  onMount(draw);
</script>

<canvas bind:this={canvas} class="timeline__waveform" aria-hidden="true"></canvas>
