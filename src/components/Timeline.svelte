<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { clamp, formatTime } from '../lib/format';

  export let duration = 0;
  export let currentTime = 0;
  export let inPoint = 0;
  export let outPoint = 0;
  export let disabled = true;

  const dispatch = createEventDispatcher<{
    seek: { seconds: number };
    setIn: { seconds: number };
    setOut: { seconds: number };
  }>();

  $: safeDuration = Math.max(duration, 0);
  $: safeOutPoint = outPoint || safeDuration;
  $: playheadPercent = safeDuration > 0 ? (currentTime / safeDuration) * 100 : 0;
  $: inPercent = safeDuration > 0 ? (inPoint / safeDuration) * 100 : 0;
  $: outPercent = safeDuration > 0 ? (safeOutPoint / safeDuration) * 100 : 100;

  function handleSeek(event: Event): void {
    const value = Number((event.target as HTMLInputElement).value);
    dispatch('seek', { seconds: clamp(value, 0, safeDuration) });
  }

  function handleIn(event: Event): void {
    const value = Number((event.target as HTMLInputElement).value);
    dispatch('setIn', { seconds: clamp(value, 0, Math.max(0, safeOutPoint - 0.1)) });
  }

  function handleOut(event: Event): void {
    const value = Number((event.target as HTMLInputElement).value);
    dispatch('setOut', { seconds: clamp(value, Math.min(inPoint + 0.1, safeDuration), safeDuration) });
  }
</script>

<section class="timeline" aria-label="Trim timeline">
  <div class="timeline__track">
    <div class="timeline__selection" style={`left: ${inPercent}%; right: ${100 - outPercent}%`}></div>
    <div class="timeline__playhead" style={`left: ${playheadPercent}%`}></div>
    <input
      aria-label="Seek"
      disabled={disabled}
      max={safeDuration}
      min="0"
      step="0.01"
      type="range"
      value={currentTime}
      on:input={handleSeek}
    />
    <input
      aria-label="In point"
      class="timeline__handle timeline__handle--in"
      disabled={disabled}
      max={safeDuration}
      min="0"
      step="0.01"
      type="range"
      value={inPoint}
      on:input={handleIn}
    />
    <input
      aria-label="Out point"
      class="timeline__handle timeline__handle--out"
      disabled={disabled}
      max={safeDuration}
      min="0"
      step="0.01"
      type="range"
      value={safeOutPoint}
      on:input={handleOut}
    />
  </div>

  <div class="timeline__meta">
    <span>In {formatTime(inPoint)}</span>
    <span>Now {formatTime(currentTime)}</span>
    <span>Out {formatTime(safeOutPoint)}</span>
  </div>
</section>
