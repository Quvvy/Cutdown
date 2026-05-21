<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import DraggablePanel from './DraggablePanel.svelte';

  export let open = false;
  export let title = 'Confirm';
  export let message = '';
  export let confirmLabel = 'Confirm';
  export let cancelLabel = 'Cancel';
  export let danger = false;

  const dispatch = createEventDispatcher<{
    close: void;
    confirm: void;
  }>();
</script>

<DraggablePanel open={open} {title} width={420} maxHeight="min(50vh, 320px)" on:close={() => dispatch('close')}>
  <p class="modal__hint">{message}</p>

  <svelte:fragment slot="footer">
    <button type="button" class="secondary" on:click={() => dispatch('close')}>{cancelLabel}</button>
    <button type="button" class:danger={danger} class="primary" on:click={() => dispatch('confirm')}>{confirmLabel}</button>
  </svelte:fragment>
</DraggablePanel>
