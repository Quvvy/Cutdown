<script lang="ts">
  import IconButton from './IconButton.svelte';
  import SvgIcon from './SvgIcon.svelte';
  import { appIcons } from '../lib/appIcons';
  import { dismissToast, toasts, type ToastKind } from '../stores/toasts';

  function kindIcon(kind: ToastKind) {
    if (kind === 'success') {
      return appIcons.success;
    }
    if (kind === 'error') {
      return appIcons.error;
    }
    return appIcons.info;
  }
</script>

<div class="toast-host" aria-live="polite" aria-relevant="additions">
  {#each $toasts as toast (toast.id)}
    <div class="toast toast--{toast.kind}" role={toast.kind === 'error' ? 'alert' : 'status'}>
      <span class="toast__icon"><SvgIcon icon={kindIcon(toast.kind)} size={18} /></span>
      <p class="toast__message">{toast.message}</p>
      <IconButton icon="dismiss" title="Dismiss" variant="mini" size={16} on:click={() => dismissToast(toast.id)} />
    </div>
  {/each}
</div>
