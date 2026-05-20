import { addIcon } from '@iconify/svelte';
import { appIcons } from './appIcons';

let registered = false;

export function registerAppIcons(): void {
  if (registered) {
    return;
  }

  for (const [name, data] of Object.entries(appIcons)) {
    addIcon(`cutdown:${name}`, data);
  }

  registered = true;
}

export function appIconId(name: keyof typeof appIcons): string {
  return `cutdown:${name}`;
}
