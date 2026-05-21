<script lang="ts">
  import { createEventDispatcher, tick } from 'svelte';
  import { clamp } from '../lib/format';

  export let open = false;
  export let title = '';
  export let width = 520;
  export let maxHeight = 'min(85vh, 720px)';

  const dispatch = createEventDispatcher<{ close: void }>();

  let posX = 80;
  let posY = 72;
  let dragging = false;
  let dragOffsetX = 0;
  let dragOffsetY = 0;

  $: if (open) {
    void centerPanel();
  }

  async function centerPanel(): Promise<void> {
    await tick();
    const margin = 16;
    posX = Math.max(margin, (window.innerWidth - width) / 2);
    posY = Math.max(margin, (window.innerHeight - 420) / 2);
  }

  function closePanel(): void {
    dragging = false;
    dispatch('close');
  }

  function startDrag(event: PointerEvent): void {
    if (event.button !== 0) {
      return;
    }

    const target = event.target as HTMLElement;
    if (target.closest('.floating-panel__close')) {
      return;
    }

    dragging = true;
    dragOffsetX = event.clientX - posX;
    dragOffsetY = event.clientY - posY;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function moveDrag(event: PointerEvent): void {
    if (!dragging) {
      return;
    }

    const maxX = Math.max(0, window.innerWidth - width - 8);
    const maxY = Math.max(0, window.innerHeight - 80);
    posX = clamp(event.clientX - dragOffsetX, 0, maxX);
    posY = clamp(event.clientY - dragOffsetY, 0, maxY);
  }

  function endDrag(event: PointerEvent): void {
    if (!dragging) {
      return;
    }

    dragging = false;
    if ((event.currentTarget as HTMLElement).hasPointerCapture(event.pointerId)) {
      (event.currentTarget as HTMLElement).releasePointerCapture(event.pointerId);
    }
  }
</script>

<svelte:window on:pointermove={moveDrag} on:pointerup={endDrag} on:pointercancel={endDrag} />

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="panel-backdrop" role="presentation" on:click={() => dispatch('close')}></div>

  <section
    class="floating-panel"
    role="dialog"
    aria-modal="true"
    aria-labelledby="floating-panel-title"
    style={`left: ${posX}px; top: ${posY}px; width: ${width}px; max-height: ${maxHeight}`}
  >
    <header
      class="floating-panel__header"
      on:pointerdown={startDrag}
    >
      <div class="floating-panel__title-wrap">
        <span class="floating-panel__drag" aria-hidden="true">⋮⋮</span>
        <h2 id="floating-panel-title">{title}</h2>
      </div>
      <button
        type="button"
        class="floating-panel__close"
        title="Close"
        on:pointerdown|stopPropagation
        on:click|stopPropagation={closePanel}
      >
        ×
      </button>
    </header>

    <div class="floating-panel__body">
      <slot />
    </div>

    {#if $$slots.footer}
      <footer class="floating-panel__footer">
        <slot name="footer" />
      </footer>
    {/if}
  </section>
{/if}
