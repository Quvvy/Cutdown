<script lang="ts">
  import { nearestThumbnailIndex } from '../lib/thumbnailPlan';
  import type { TimelineSegment } from '../stores/editor';

  /** Data URLs, null where a frame hasn't loaded yet. */
  export let thumbnails: (string | null)[] = [];
  export let sourceDuration = 0;
  export let segments: TimelineSegment[] = [];
  export let pixelsPerSecond = 1;
  export let width = 0;
  export let height = 0;
  /** Frame width / height, so tiles keep the video's shape (vertical video stays narrow). */
  export let aspect = 16 / 9;

  let canvas: HTMLCanvasElement | undefined;
  const images = new Map<string, HTMLImageElement>();

  function imageFor(url: string): HTMLImageElement | null {
    let image = images.get(url);
    if (!image) {
      image = new Image();
      image.onload = () => draw();
      image.src = url;
      images.set(url, image);
    }
    return image.complete && image.naturalWidth > 0 ? image : null;
  }

  function draw(): void {
    if (!canvas || width <= 0 || height <= 0) {
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
    if (!thumbnails.length || sourceDuration <= 0) {
      return;
    }

    // Prefer a loaded frame's shape: rotated phone video reports sideways dimensions,
    // but ffmpeg hands back upright frames.
    const sample = thumbnails.map((url) => (url ? images.get(url) : undefined)).find((image) => image?.naturalWidth);
    const frameAspect = sample ? sample.naturalWidth / sample.naturalHeight : aspect > 0 ? aspect : 16 / 9;
    const tileWidth = Math.max(16, height * frameAspect);
    let sequenceStart = 0;

    // Tile each segment from its own start, so every segment opens on its first frame.
    for (const segment of segments) {
      const segmentSeconds = segment.sourceEnd - segment.sourceStart;
      const left = sequenceStart * pixelsPerSecond;
      const right = Math.min(width, (sequenceStart + segmentSeconds) * pixelsPerSecond);
      sequenceStart += segmentSeconds;
      if (right <= left) {
        continue;
      }

      context.save();
      context.beginPath();
      context.rect(left, 0, right - left, height);
      context.clip();
      for (let x = left; x < right; x += tileWidth) {
        const sourceTime = segment.sourceStart + (x - left + tileWidth / 2) / pixelsPerSecond;
        const url = thumbnails[nearestThumbnailIndex(sourceTime, sourceDuration, thumbnails.length)];
        const image = url ? imageFor(url) : null;
        if (image) {
          context.drawImage(image, x, 0, tileWidth, height);
        }
      }
      context.restore();
    }
  }

  $: thumbnails, sourceDuration, segments, pixelsPerSecond, width, height, aspect, draw();
</script>

<canvas bind:this={canvas} class="timeline__filmstrip" aria-hidden="true"></canvas>
