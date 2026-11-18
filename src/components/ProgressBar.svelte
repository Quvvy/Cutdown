<script lang="ts">
  import type { ExportStatus } from '../stores/editor';

  export let active = false;
  export let label = '';
  export let percent: number | null = null;
  export let state: ExportStatus['state'] = 'idle';

  $: width = percent === null ? null : `${Math.max(0, Math.min(100, percent))}%`;
  $: showBar = active || state === 'running';
</script>

<div
  class="progress"
  class:progress--running={state === 'running'}
  class:progress--idle={state === 'idle'}
  class:active
  role="progressbar"
  aria-label={label || 'Export progress'}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={percent === null ? undefined : Math.round(percent)}
>
  {#if showBar}
    <div class="progress__track">
      <div
        class="progress__bar"
        class:indeterminate={active && percent === null}
        style:width={width ?? undefined}
      ></div>
    </div>
  {/if}
  <span class="progress__label" title={label}>{label}</span>
</div>
