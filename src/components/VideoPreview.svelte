<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatTime } from '../lib/format';

  export let src: string | null = null;
  export let currentTime = 0;

  let video: HTMLVideoElement;
  const dispatch = createEventDispatcher<{
    timeupdate: { currentTime: number };
    metadata: { duration: number };
  }>();

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
    dispatch('metadata', { duration: video.duration || 0 });
  }

  function handleTimeUpdate(): void {
    dispatch('timeupdate', { currentTime: video.currentTime });
  }
</script>

<section class="video-preview" aria-label="Video preview">
  {#if src}
    <video
      bind:this={video}
      src={src}
      preload="metadata"
      on:click={togglePlayback}
      on:loadedmetadata={handleLoadedMetadata}
      on:timeupdate={handleTimeUpdate}
    >
      <track kind="captions" />
    </video>
    <div class="timecode">{formatTime(currentTime)}</div>
  {:else}
    <div class="video-preview__empty">
      <strong>No clip loaded</strong>
      <span>Choose a video file to start trimming.</span>
    </div>
  {/if}
</section>
