<script lang="ts">
  export let active = false;
  export let label = '';
</script>

<div class:active class="progress" aria-live="polite">
  <div class="progress__bar"></div>
  <span>{label}</span>
</div>
