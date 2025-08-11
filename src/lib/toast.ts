import { writable } from 'svelte/store';

export type ToastType = 'success' | 'info' | 'warning' | 'error';

export type Toast = {
  id: number;
  message: string;
  type: ToastType;
  timeout: number;
};

function createToastStore() {
  const { subscribe, update } = writable<Toast[]>([]);
  let nextId = 1;

  function removeToast(id: number) {
    update((items) => items.filter((t) => t.id !== id));
  }

  function showToast(message: string, type: ToastType = 'info', timeout = 2600) {
    const id = nextId++;
    const toast: Toast = { id, message, type, timeout };
    update((items) => [...items, toast]);
    // Auto dismiss
    window.setTimeout(() => removeToast(id), timeout);
  }

  return {
    subscribe,
    showToast,
    removeToast,
  } as const;
}

export const toasts = createToastStore();

export const showToast = (message: string, type: ToastType = 'info', timeout?: number) =>
  toasts.showToast(message, type, timeout);

export const removeToast = (id: number) => toasts.removeToast(id);



