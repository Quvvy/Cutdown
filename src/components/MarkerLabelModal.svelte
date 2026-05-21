<script lang="ts">
  import { createEventDispatcher } from 'svelte';

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

{#if open}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="modal-backdrop" on:click={() => dispatch('close')}>
    <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
    <section class="modal modal--narrow" aria-label="Marker label" on:click|stopPropagation>
      <header>
        <h2>Marker label</h2>
        <button type="button" class="icon-button" title="Close" on:click={() => dispatch('close')}>Close</button>
      </header>
      <div class="modal__body">
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
      </div>
      <footer>
        <button type="button" class="secondary" on:click={() => dispatch('close')}>Cancel</button>
        <button type="button" on:click={save}>Save</button>
      </footer>
    </section>
  </div>
{/if}
