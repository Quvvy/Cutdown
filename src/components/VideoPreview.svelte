<script lang="ts">
  import { createEventDispatcher, onMount, tick } from 'svelte';
  import { clamp, formatTime } from '../lib/format';
  import type { NormalizedCropRect } from '../lib/types';

  export let src: string | null = null;
  export let currentTime = 0;
  export let loopEnabled = false;
  export let loopStart: number | null = null;
  export let loopEnd: number | null = null;
  export let cropEnabled = false;
  export let cropRect: NormalizedCropRect = { x: 0, y: 0, width: 1, height: 1 };
  export let lockAspectRatio = false;
  export let lockedAspectRatio: number | null = null;
  export let volume = 1;
  export let playbackRate = 1;

  let video: HTMLVideoElement;
  let viewport: HTMLDivElement;
  let stage: HTMLDivElement;
  let loadError = '';
  let previousSrc: string | null = null;
  let dragMode: 'move' | 'resize' | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  let dragStartRect: NormalizedCropRect = cropRect;
  let videoWidth = 0;
  let videoHeight = 0;
  let viewportWidth = 0;
  let viewportHeight = 0;
  let zoomFactor = 1;
  let panX = 0;
  let panY = 0;
  let isPanning = false;
  let panPointerId: number | null = null;
  let panStartX = 0;
  let panStartY = 0;
  let panOriginX = 0;
  let panOriginY = 0;
  let userAdjustedView = false;
  let suppressNextPauseEvent = false;

  const dispatch = createEventDispatcher<{
    timeupdate: { currentTime: number };
    metadata: { duration: number };
    previewready: void;
    playstate: { playing: boolean };
    ended: void;
    error: { message: string };
    cropChange: { rect: NormalizedCropRect };
  }>();

  let previewReadyForSrc: string | null = null;

  $: if (src !== previousSrc) {
    previousSrc = src;
    previewReadyForSrc = null;
    loadError = '';
    userAdjustedView = false;
    resetView();
  }

  $: if (video) {
    video.volume = clamp(volume, 0, 1);
    video.playbackRate = clamp(playbackRate, 0.25, 2);
  }

  $: fitScale =
    videoWidth > 0 && videoHeight > 0 && viewportWidth > 0 && viewportHeight > 0
      ? Math.min(viewportWidth / videoWidth, viewportHeight / videoHeight)
      : 1;
  $: displayScale = fitScale * zoomFactor;
  $: stageWidth = videoWidth > 0 ? videoWidth * displayScale : 0;
  $: stageHeight = videoHeight > 0 ? videoHeight * displayScale : 0;
  $: hasLayout =
    videoWidth > 0 &&
    videoHeight > 0 &&
    viewportWidth > 0 &&
    viewportHeight > 0;
  $: stageTransform = hasLayout
    ? `translate(${(viewportWidth - stageWidth) / 2 + panX}px, ${(viewportHeight - stageHeight) / 2 + panY}px) scale(${displayScale})`
    : undefined;

  $: cropStyle = cropEnabled
    ? `inset(${(cropRect.y * 100).toFixed(2)}% ${((1 - cropRect.x - cropRect.width) * 100).toFixed(2)}% ${((1 - cropRect.y - cropRect.height) * 100).toFixed(2)}% ${(cropRect.x * 100).toFixed(2)}%)`
    : 'inset(0)';

  function measureViewport(): void {
    if (!viewport) {
      return;
    }

    viewportWidth = viewport.clientWidth;
    viewportHeight = viewport.clientHeight;
  }

  export function remeasureViewport(): void {
    measureViewport();
    if (!userAdjustedView) {
      void fitToView();
    }
  }

  onMount(() => {
    measureViewport();
    const observer = new ResizeObserver(() => {
      measureViewport();
      if (!userAdjustedView && videoWidth > 0) {
        void fitToView();
      }
    });
    if (viewport) {
      observer.observe(viewport);
    }

    return () => observer.disconnect();
  });

  export function resetView(): void {
    zoomFactor = 1;
    panX = 0;
    panY = 0;
    isPanning = false;
    panPointerId = null;
  }

  export async function fitToView(): Promise<void> {
    userAdjustedView = false;
    resetView();
    await tick();
    measureViewport();
  }

  export function zoomIn(): void {
    userAdjustedView = true;
    zoomFactor = Math.min(8, zoomFactor * 1.25);
  }

  export function zoomOut(): void {
    userAdjustedView = true;
    zoomFactor = Math.max(0.25, zoomFactor / 1.25);
    if (zoomFactor <= 1.01) {
      panX = 0;
      panY = 0;
      userAdjustedView = false;
    }
  }

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
      playPlayback();
    } else {
      pausePlayback();
    }
  }

  export function playPlayback(): void {
    if (!video) {
      return;
    }

    void video.play().catch(() => {
      dispatch('playstate', { playing: false });
    });
  }

  export function pausePlayback(options: { emit?: boolean } = {}): void {
    if (!video) {
      return;
    }

    suppressNextPauseEvent = options.emit === false && !video.paused;
    video.pause();
    if (options.emit !== false) {
      dispatch('playstate', { playing: false });
    }
  }

  export function isReady(): boolean {
    return Boolean(src && previewReadyForSrc === src);
  }

  async function handleLoadedMetadata(): Promise<void> {
    loadError = '';
    videoWidth = video.videoWidth || 0;
    videoHeight = video.videoHeight || 0;
    resetView();
    await tick();
    measureViewport();
    dispatch('metadata', { duration: video.duration || 0 });
  }

  function handleCanPlay(): void {
    if (!src || src === previewReadyForSrc) {
      return;
    }

    previewReadyForSrc = src;
    dispatch('previewready');
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

  function handlePlay(): void {
    dispatch('playstate', { playing: true });
  }

  function handlePause(): void {
    if (suppressNextPauseEvent) {
      suppressNextPauseEvent = false;
      return;
    }
    dispatch('playstate', { playing: false });
  }

  function handleEnded(): void {
    suppressNextPauseEvent = false;
    dispatch('ended');
  }

  function handleError(): void {
    loadError =
      'The preview could not decode this file. Try an H.264/AAC MP4, or export/remux the source first.';
    dispatch('error', { message: loadError });
  }

  function clampRect(rect: NormalizedCropRect): NormalizedCropRect {
    let width = Math.min(1, Math.max(0.05, rect.width));
    let height = Math.min(1, Math.max(0.05, rect.height));

    if (lockAspectRatio && lockedAspectRatio && lockedAspectRatio > 0 && videoWidth > 0 && videoHeight > 0) {
      const maxNormHeight = (width * videoWidth) / (videoHeight * lockedAspectRatio);
      if (height > maxNormHeight) {
        height = maxNormHeight;
      }
      const maxNormWidth = (height * videoHeight * lockedAspectRatio) / videoWidth;
      if (width > maxNormWidth) {
        width = maxNormWidth;
      }
      height = (width * videoWidth) / (videoHeight * lockedAspectRatio);
    }

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
    const bounds = stage.getBoundingClientRect();
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
    if (!dragMode || !stage) {
      return;
    }

    const bounds = stage.getBoundingClientRect();
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

    let width = dragStartRect.width + dx;
    let height = dragStartRect.height + dy;

    if (lockAspectRatio && lockedAspectRatio && lockedAspectRatio > 0 && videoWidth > 0 && videoHeight > 0) {
      const absDx = Math.abs(dx);
      const absDy = Math.abs(dy);
      if (absDx >= absDy) {
        height = (width * videoWidth) / (videoHeight * lockedAspectRatio);
      } else {
        width = (height * videoHeight * lockedAspectRatio) / videoWidth;
      }
    }

    emitCrop({
      x: dragStartRect.x,
      y: dragStartRect.y,
      width,
      height,
    });
  }

  function onCropPointerUp(event: PointerEvent): void {
    if (!dragMode) {
      return;
    }

    dragMode = null;
    (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
  }

  function canPan(): boolean {
    return zoomFactor > 1.01 || Math.abs(panX) > 0.5 || Math.abs(panY) > 0.5;
  }

  function onViewportPointerDown(event: PointerEvent): void {
    if (cropEnabled) {
      return;
    }

    const isMiddleButton = event.button === 1;
    const isAltDrag = event.button === 0 && event.altKey;
    if (!isMiddleButton && !isAltDrag) {
      return;
    }

    event.preventDefault();
    isPanning = true;
    panPointerId = event.pointerId;
    panStartX = event.clientX;
    panStartY = event.clientY;
    panOriginX = panX;
    panOriginY = panY;
    viewport.setPointerCapture(event.pointerId);
  }

  function onViewportPointerMove(event: PointerEvent): void {
    if (!isPanning || event.pointerId !== panPointerId) {
      return;
    }

    userAdjustedView = true;
    panX = panOriginX + (event.clientX - panStartX);
    panY = panOriginY + (event.clientY - panStartY);
  }

  function onViewportPointerUp(event: PointerEvent): void {
    if (!isPanning || event.pointerId !== panPointerId) {
      return;
    }

    isPanning = false;
    panPointerId = null;
    viewport.releasePointerCapture(event.pointerId);
  }

  function onViewportWheel(event: WheelEvent): void {
    if (!videoWidth || cropEnabled) {
      return;
    }

    event.preventDefault();
    const factor = event.deltaY < 0 ? 1.12 : 1 / 1.12;
    const nextZoom = clamp(zoomFactor * factor, 0.25, 8);
    if (nextZoom <= 1.01) {
      zoomFactor = nextZoom;
      panX = 0;
      panY = 0;
      userAdjustedView = false;
      return;
    }

    userAdjustedView = true;
    zoomFactor = nextZoom;
  }

  function onViewportDoubleClick(event: MouseEvent): void {
    if (cropEnabled) {
      return;
    }

    event.preventDefault();
    fitToView();
  }
</script>

<section class="video-preview" aria-label="Video preview">
  {#if src}
  <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div
      class="video-preview__viewport"
      class:video-preview__viewport--panning={isPanning}
      class:video-preview__viewport--pannable={canPan()}
      bind:this={viewport}
      on:pointerdown={onViewportPointerDown}
      on:pointermove={onViewportPointerMove}
      on:pointerup={onViewportPointerUp}
      on:pointercancel={onViewportPointerUp}
      on:wheel={onViewportWheel}
      on:dblclick={onViewportDoubleClick}
    >
      <div
        class="video-preview__stage"
        class:video-preview__stage--zoomed={hasLayout}
        bind:this={stage}
        style:width={hasLayout && videoWidth > 0 ? `${videoWidth}px` : undefined}
        style:height={hasLayout && videoHeight > 0 ? `${videoHeight}px` : undefined}
        style:transform={stageTransform}
      >
        <video
          bind:this={video}
          src={src}
          preload="metadata"
          style:clip-path={cropEnabled ? cropStyle : undefined}
          on:click={togglePlayback}
          on:error={handleError}
          on:loadedmetadata={handleLoadedMetadata}
          on:canplay={handleCanPlay}
          on:play={handlePlay}
          on:pause={handlePause}
          on:ended={handleEnded}
          on:timeupdate={handleTimeUpdate}
        >
          <track kind="captions" />
        </video>
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
      <div class="timecode">{formatTime(currentTime)}</div>
      {#if canPan()}
        <div class="video-preview__hint">Alt+drag to pan · scroll to zoom · double-click to fit</div>
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
