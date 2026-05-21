<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import DraggablePanel from './DraggablePanel.svelte';

  export let open = false;
  export let missingPath = '';
  export let projectName = '';

  const dispatch = createEventDispatcher<{
    close: void;
    choose: void;
  }>();
</script>

<DraggablePanel open={open} title="Source file missing" width={460} on:close={() => dispatch('close')}>
  {#if projectName}
    <p class="panel-section__lead">Project <strong>{projectName}</strong> references a file that could not be found.</p>
  {/if}
  <p class="modal__hint modal__hint--error">{missingPath}</p>
  <p class="modal__hint">Choose the video file on disk to relink this project.</p>

  <svelte:fragment slot="footer">
    <button type="button" class="secondary" on:click={() => dispatch('close')}>Cancel</button>
    <button type="button" class="primary" on:click={() => dispatch('choose')}>Choose source file</button>
  </svelte:fragment>
</DraggablePanel>
