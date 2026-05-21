<script lang="ts">
  import { dismissToast, toasts, type ToastKind } from '../stores/toasts';

  function kindLabel(kind: ToastKind): string {
    switch (kind) {
      case 'success':
        return 'Success';
      case 'error':
        return 'Error';
      default:
        return 'Notice';
    }
  }
</script>

<div class="toast-host" aria-live="polite" aria-relevant="additions">
  {#each $toasts as toast (toast.id)}
    <div class="toast toast--{toast.kind}" role="status">
      <span class="toast__kind">{kindLabel(toast.kind)}</span>
      <p class="toast__message">{toast.message}</p>
      <button type="button" class="toast__dismiss" title="Dismiss" on:click={() => dismissToast(toast.id)}>×</button>
    </div>
  {/each}
</div>
