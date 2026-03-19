<script>
  import { toasts } from "./toastStore.js";
</script>

{#each $toasts as toast (toast.id)}
  <div class="toast toast-{toast.type}">
    {toast.message}
  </div>
{/each}

<style>
  .toast {
    position: fixed;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 20px;
    border-radius: 8px;
    font-size: 13px;
    font-weight: 500;
    z-index: 9999;
    animation: toastIn 0.25s ease;
    pointer-events: none;
  }

  .toast-success {
    background: rgba(34, 197, 94, 0.9);
    color: #fff;
  }

  .toast-error {
    background: rgba(239, 68, 68, 0.9);
    color: #fff;
  }

  .toast-info {
    background: rgba(137, 180, 250, 0.9);
    color: #1a1b23;
  }

  @keyframes toastIn {
    from {
      opacity: 0;
      transform: translateX(-50%) translateY(-12px);
    }
    to {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }
  }
</style>
