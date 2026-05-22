import { tick } from 'svelte';

export type MenuAnchor = { x: number; y: number };

/** Position a fixed menu so it stays fully inside the viewport (flip/clamp). */
export function placeMenu(node: HTMLElement, anchor: MenuAnchor): { update: (anchor: MenuAnchor) => void } {
  async function apply(next: MenuAnchor): Promise<void> {
    await tick();
    const margin = 8;
    const rect = node.getBoundingClientRect();
    let left = next.x;
    let top = next.y;

    if (left + rect.width > window.innerWidth - margin) {
      left = Math.max(margin, window.innerWidth - rect.width - margin);
    }

    if (top + rect.height > window.innerHeight - margin) {
      top = Math.max(margin, next.y - rect.height);
    }

    if (top < margin) {
      top = margin;
    }

    if (left < margin) {
      left = margin;
    }

    node.style.left = `${left}px`;
    node.style.top = `${top}px`;
  }

  void apply(anchor);

  return {
    update(next: MenuAnchor) {
      void apply(next);
    },
  };
}
