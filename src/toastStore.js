import { writable } from "svelte/store";

let id = 0;

function createToastStore() {
  const { subscribe, update } = writable([]);

  return {
    subscribe,
    add(message, type = "success", duration = 2000) {
      const toast = { id: ++id, message, type, visible: true };
      update((toasts) => [...toasts, toast]);
      setTimeout(() => {
        update((toasts) => toasts.filter((t) => t.id !== toast.id));
      }, duration);
    },
  };
}

export const toasts = createToastStore();

export function addToast(message, type = "success", duration = 2000) {
  toasts.add(message, type, duration);
}
