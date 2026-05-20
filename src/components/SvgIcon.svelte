<script lang="ts">
  import type { IconifyIcon } from '@iconify/svelte';

  export let icon: IconifyIcon;
  export let size = 18;
</script>

<svg
  class="icon-btn__glyph"
  xmlns="http://www.w3.org/2000/svg"
  width={size}
  height={size}
  viewBox="0 0 {icon.width} {icon.height}"
  aria-hidden="true"
>
  {@html icon.body}
</svg>
