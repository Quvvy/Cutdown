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
  let panelEl: HTMLElement;
  let previousFocus: HTMLElement | null = null;
  let wasOpen = false;

  $: if (open && !wasOpen) {
    wasOpen = true;
    void openPanel();
  }

  $: if (!open && wasOpen) {
    wasOpen = false;
  }

  async function openPanel(): Promise<void> {
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    await tick();
    const margin = 16;
    posX = Math.max(margin, (window.innerWidth - width) / 2);
    posY = Math.max(margin, (window.innerHeight - 420) / 2);
    focusFirstElement();
  }

  function closePanel(): void {
    dragging = false;
    dispatch('close');
    previousFocus?.focus();
  }

  function focusableElements(): HTMLElement[] {
    if (!panelEl) {
      return [];
    }

    return Array.from(
      panelEl.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      ),
    ).filter((element) => !element.hasAttribute('disabled') && element.tabIndex >= 0);
  }

  function focusFirstElement(): void {
    const [first] = focusableElements();
    (first ?? panelEl)?.focus();
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      closePanel();
      return;
    }

    if (event.key !== 'Tab') {
      return;
    }

    const focusable = focusableElements();
    if (focusable.length === 0) {
      event.preventDefault();
      panelEl?.focus();
      return;
    }

    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
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
  <div class="panel-backdrop" role="presentation" on:click={closePanel}></div>

  <!-- svelte-ignore a11y-no-noninteractive-element-interactions -->
  <section
    bind:this={panelEl}
    class="floating-panel"
    role="dialog"
    aria-modal="true"
    aria-labelledby="floating-panel-title"
    tabindex="-1"
    style:left={`${posX}px`}
    style:top={`${posY}px`}
    style:width={`${width}px`}
    style:max-height={maxHeight}
    on:keydown={handleKeydown}
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
