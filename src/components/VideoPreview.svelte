<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { clamp, formatTime } from '../lib/format';
  import type { NormalizedCropRect } from '../lib/types';

  export let src: string | null = null;
  export let currentTime = 0;
  export let loopEnabled = false;
  export let loopStart: number | null = null;
  export let loopEnd: number | null = null;
  export let cropEnabled = false;
  export let cropRect: NormalizedCropRect = { x: 0, y: 0, width: 1, height: 1 };
  export let volume = 1;

  let video: HTMLVideoElement;
  let frame: HTMLDivElement;
  let loadError = '';
  let previousSrc: string | null = null;
  let dragMode: 'move' | 'resize' | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let dragStartRect: NormalizedCropRect = cropRect;

  const dispatch = createEventDispatcher<{
    timeupdate: { currentTime: number };
    metadata: { duration: number };
    error: { message: string };
    cropChange: { rect: NormalizedCropRect };
  }>();

  $: if (src !== previousSrc) {
    previousSrc = src;
    loadError = '';
  }

  $: if (video) {
    video.volume = clamp(volume, 0, 1);
  }

  $: cropStyle = cropEnabled
    ? `inset(${(cropRect.y * 100).toFixed(2)}% ${((1 - cropRect.x - cropRect.width) * 100).toFixed(2)}% ${((1 - cropRect.y - cropRect.height) * 100).toFixed(2)}% ${(cropRect.x * 100).toFixed(2)}%)`
    : 'inset(0)';

  export function seekTo(seconds: number): void {
    if (!video) {
      return;
    }

    video.currentTime = seconds;
    dispatch('timeupdate', { currentTime: video.currentTime });
  }

  export function togglePlayback(): void {
    if (!video) {
      return;
    }

    if (video.paused) {
      void video.play();
    } else {
      video.pause();
    }
  }

  function handleLoadedMetadata(): void {
    loadError = '';
    dispatch('metadata', { duration: video.duration || 0 });
  }

  function handleTimeUpdate(): void {
    if (
      loopEnabled &&
      loopStart !== null &&
      loopEnd !== null &&
      loopEnd > loopStart + 0.05 &&
      video.currentTime >= loopEnd - 0.02
    ) {
      video.currentTime = loopStart;
    }

    dispatch('timeupdate', { currentTime: video.currentTime });
  }

  function handleError(): void {
    loadError =
      'The preview could not decode this file. Try an H.264/AAC MP4, or export/remux the source first.';
    dispatch('error', { message: loadError });
  }

  function clampRect(rect: NormalizedCropRect): NormalizedCropRect {
    const width = Math.min(1, Math.max(0.05, rect.width));
    const height = Math.min(1, Math.max(0.05, rect.height));
    const x = Math.min(1 - width, Math.max(0, rect.x));
    const y = Math.min(1 - height, Math.max(0, rect.y));
    return { x, y, width, height };
  }

  function emitCrop(rect: NormalizedCropRect): void {
    const next = clampRect(rect);
    cropRect = next;
    dispatch('cropChange', { rect: next });
  }

  function pointerToNormalized(event: PointerEvent): { x: number; y: number } {
    const bounds = frame.getBoundingClientRect();
    return {
      x: (event.clientX - bounds.left) / bounds.width,
      y: (event.clientY - bounds.top) / bounds.height,
    };
  }

  function onCropPointerDown(event: PointerEvent, mode: 'move' | 'resize'): void {
    if (!cropEnabled) {
      return;
    }

    event.stopPropagation();
    dragMode = mode;
    dragStartX = event.clientX;
    dragStartY = event.clientY;
    dragStartRect = { ...cropRect };
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function onCropPointerMove(event: PointerEvent): void {
    if (!dragMode || !frame) {
      return;
    }

    const bounds = frame.getBoundingClientRect();
    const dx = (event.clientX - dragStartX) / bounds.width;
    const dy = (event.clientY - dragStartY) / bounds.height;

    if (dragMode === 'move') {
      emitCrop({
        x: dragStartRect.x + dx,
        y: dragStartRect.y + dy,
        width: dragStartRect.width,
        height: dragStartRect.height,
      });
      return;
    }

    emitCrop({
      x: dragStartRect.x,
      y: dragStartRect.y,
      width: dragStartRect.width + dx,
      height: dragStartRect.height + dy,
    });
  }

  function onCropPointerUp(event: PointerEvent): void {
    if (!dragMode) {
      return;
    }

    dragMode = null;
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
  }
</script>

<section class="video-preview" aria-label="Video preview">
  {#if src}
    <div class="video-preview__frame" bind:this={frame}>
      <video
        bind:this={video}
        src={src}
        preload="metadata"
        style:clip-path={cropEnabled ? cropStyle : undefined}
        on:click={togglePlayback}
        on:error={handleError}
        on:loadedmetadata={handleLoadedMetadata}
        on:timeupdate={handleTimeUpdate}
      >
        <track kind="captions" />
      </video>
      <div class="timecode">{formatTime(currentTime)}</div>
      {#if cropEnabled}
        <!-- svelte-ignore a11y-no-static-element-interactions -->
        <div
          class="crop-overlay"
          style={`left:${cropRect.x * 100}%;top:${cropRect.y * 100}%;width:${cropRect.width * 100}%;height:${cropRect.height * 100}%;`}
          on:pointerdown={(event) => onCropPointerDown(event, 'move')}
          on:pointermove={onCropPointerMove}
          on:pointerup={onCropPointerUp}
          on:pointercancel={onCropPointerUp}
        >
          <button
            type="button"
            class="crop-overlay__handle"
            aria-label="Resize crop"
            on:pointerdown={(event) => onCropPointerDown(event, 'resize')}
          ></button>
        </div>
      {/if}
    </div>
    {#if loadError}
      <div class="video-preview__error">{loadError}</div>
    {/if}
  {:else}
    <div class="video-preview__empty">
      <strong>No clip loaded</strong>
      <span>Choose a video file to start trimming.</span>
    </div>
  {/if}
</section>
