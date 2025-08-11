<script lang="ts">
  import { toasts, type Toast as ToastType } from '$lib/toast';

  let items: ToastType[] = [];
  const unsubscribe = toasts.subscribe((v) => (items = v));
  import { onDestroy } from 'svelte';
  onDestroy(() => unsubscribe());
</script>

<div class="toast-container" aria-live="polite" aria-atomic="true">
  {#each items as t (t.id)}
    <div class="toast {t.type}">
      <div class="toast-dot" />
      <div class="toast-content">{t.message}</div>
    </div>
  {/each}
  {#if items.length === 0}
    <!-- keep layer for pointer-events consistency -->
    <div class="placeholder" />
  {/if}
</div>

<style>
  .toast-container {
    position: fixed;
    top: 16px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    z-index: 9999;
    pointer-events: none;
  }

  .toast {
    pointer-events: auto;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: unset;
    max-width: min(60vw, 560px);
    padding: 12px 14px;
    background: #ffffff;
    border: 1px solid var(--border-primary);
    border-radius: 12px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.12);
    color: var(--text-default);
    animation: toast-in 160ms ease-out;
  }

  @keyframes toast-in {
    from { opacity: 0; transform: translateY(-8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .toast-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--status-info);
  }

  .toast.success .toast-dot { background: var(--status-success); }
  .toast.info .toast-dot { background: var(--status-info); }
  .toast.warning .toast-dot { background: var(--status-warning); }
  .toast.error .toast-dot { background: var(--status-error); }

  .toast-content {
    font-size: 14px;
    line-height: 1.4;
    white-space: pre-wrap;
  }
</style>


