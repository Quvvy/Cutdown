<script lang="ts">
  import type { IconifyIcon } from '@iconify/svelte';
  import { appIcons, type AppIconName } from '../lib/appIcons';
  import SvgIcon from './SvgIcon.svelte';

  export let icon: AppIconName | IconifyIcon;
  export let title: string;
  export let disabled = false;
  export let active = false;
  export let pressed: boolean | undefined = undefined;
  export let variant: 'tool' | 'secondary' | 'icon' | 'mini' | 'primary' = 'tool';
  export let size = 18;
  export let showLabel = false;

  $: iconData = typeof icon === 'string' ? appIcons[icon] : icon;
  $: className = `icon-btn icon-btn--${variant}${showLabel ? ' icon-btn--labeled' : ''}`;
</script>

<button
  type="button"
  class={className}
  class:active
  {title}
  aria-label={title}
  {disabled}
  aria-pressed={pressed}
  on:click
>
  {#if iconData}
    <SvgIcon icon={iconData} {size} />
  {/if}
  {#if showLabel}
    <span class="icon-btn__label"><slot>{title}</slot></span>
  {/if}
</button>
