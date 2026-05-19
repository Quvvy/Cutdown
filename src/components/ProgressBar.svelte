<script lang="ts">
  export let active = false;
  export let label = '';
  export let percent: number | null = null;

  $: width = percent === null ? null : `${Math.max(0, Math.min(100, percent))}%`;
</script>

<div class:active class="progress" aria-live="polite">
  <div
    class="progress__bar"
    class:indeterminate={active && percent === null}
    style={width ? `width: ${width}` : undefined}
  ></div>
  <span>{label}</span>
</div>
