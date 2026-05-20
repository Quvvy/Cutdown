import type { IconifyIcon } from '@iconify/svelte';

/** Turn a simple single-path (or multi-path) SVG file into an Iconify icon body. */
export function iconFromSvg(svg: string): IconifyIcon {
  const viewBoxMatch = svg.match(/viewBox=["']([^"']+)["']/);
  let width = 20;
  let height = 20;

  if (viewBoxMatch) {
    const parts = viewBoxMatch[1].trim().split(/\s+/).map(Number);
    if (parts.length === 4 && parts.every((value) => Number.isFinite(value))) {
      width = parts[2];
      height = parts[3];
    }
  } else {
    const widthMatch = svg.match(/\bwidth=["'](\d+)/);
    const heightMatch = svg.match(/\bheight=["'](\d+)/);
    if (widthMatch) width = Number(widthMatch[1]);
    if (heightMatch) height = Number(heightMatch[1]);
  }

  const paths = [...svg.matchAll(/<path\b[^>]*\bd=["']([^"']+)["'][^>]*>/gi)];
  const body = paths
    .map((match) => `<path fill="currentColor" d="${match[1]}"/>`)
    .join('');

  if (!body) {
    throw new Error('SVG icon has no <path d="..."> elements');
  }

  return { width, height, body };
}
