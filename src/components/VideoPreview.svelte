<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatTime } from '../lib/format';

  export let src: string | null = null;
  export let currentTime = 0;
  export let loopEnabled = false;
  export let loopStart: number | null = null;
  export let loopEnd: number | null = null;

  let video: HTMLVideoElement;
  let loadError = '';
  let previousSrc: string | null = null;
  const dispatch = createEventDispatcher<{
    timeupdate: { currentTime: number };
    metadata: { duration: number };
    error: { message: string };
  }>();

  $: if (src !== previousSrc) {
    previousSrc = src;
    loadError = '';
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
</script>

<section class="video-preview" aria-label="Video preview">
  {#if src}
    <video
      bind:this={video}
      src={src}
      preload="metadata"
      on:click={togglePlayback}
      on:error={handleError}
      on:loadedmetadata={handleLoadedMetadata}
      on:timeupdate={handleTimeUpdate}
    >
      <track kind="captions" />
    </video>
    <div class="timecode">{formatTime(currentTime)}</div>
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
