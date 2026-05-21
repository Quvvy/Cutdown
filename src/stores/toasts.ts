import { writable } from 'svelte/store';

export type ToastKind = 'info' | 'success' | 'error';

export type Toast = {
  id: string;
  kind: ToastKind;
  message: string;
  createdAt: number;
};

const DISMISS_MS = 4500;

export const toasts = writable<Toast[]>([]);

const timers = new Map<string, ReturnType<typeof setTimeout>>();

function scheduleDismiss(id: string): void {
  const existing = timers.get(id);
  if (existing) {
    clearTimeout(existing);
  }

  const timer = setTimeout(() => {
    dismissToast(id);
    timers.delete(id);
  }, DISMISS_MS);
  timers.set(id, timer);
}

export function pushToast(message: string, kind: ToastKind = 'info'): string {
  const id = crypto.randomUUID();
  const toast: Toast = { id, kind, message, createdAt: Date.now() };
  toasts.update((items) => [...items.slice(-4), toast]);
  scheduleDismiss(id);
  return id;
}

export function dismissToast(id: string): void {
  const timer = timers.get(id);
  if (timer) {
    clearTimeout(timer);
    timers.delete(id);
  }
  toasts.update((items) => items.filter((item) => item.id !== id));
}

export function clearToasts(): void {
  for (const timer of timers.values()) {
    clearTimeout(timer);
  }
  timers.clear();
  toasts.set([]);
}
