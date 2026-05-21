<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import DraggablePanel from './DraggablePanel.svelte';

  export let open = false;
  export let initialLabel = '';

  let label = '';

  const dispatch = createEventDispatcher<{
    close: void;
    save: { label: string };
  }>();

  $: if (open) {
    label = initialLabel;
  }

  function save(): void {
    dispatch('save', { label: label.trim() });
    dispatch('close');
  }
</script>

<DraggablePanel open={open} title="Marker label" width={380} maxHeight="min(40vh, 260px)" on:close={() => dispatch('close')}>
  <label class="modal__stack">
    <span>Label</span>
    <input
      type="text"
      class="modal__text-input"
      bind:value={label}
      maxlength="64"
      placeholder="Marker name"
      on:keydown={(event) => {
        if (event.key === 'Enter') {
          event.preventDefault();
          save();
        }
      }}
    />
  </label>

  <svelte:fragment slot="footer">
    <button type="button" class="secondary" on:click={() => dispatch('close')}>Cancel</button>
    <button type="button" class="primary" on:click={save}>Save</button>
  </svelte:fragment>
</DraggablePanel>
