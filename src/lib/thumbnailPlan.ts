// Which frames the timeline filmstrip samples, and in what order they're fetched.

/** About two thumbnails per second for short clips, capped for long ones. */
export function thumbnailCount(durationSeconds: number): number {
  if (!(durationSeconds > 0)) {
    return 0;
  }
  return Math.min(60, Math.max(12, Math.round(durationSeconds * 2)));
}

/** Evenly spaced sample times, each at the middle of its slice of the clip. */
export function thumbnailTimes(durationSeconds: number, count: number): number[] {
  if (!(durationSeconds > 0) || count <= 0) {
    return [];
  }
  const slice = durationSeconds / count;
  return Array.from({ length: count }, (_, index) => (index + 0.5) * slice);
}

/**
 * Fetch order that covers the whole strip early and then fills gaps: first, last,
 * middle, then the midpoints of the remaining gaps, so the filmstrip sharpens
 * evenly instead of loading left to right.
 */
export function coarseToFineOrder(count: number): number[] {
  if (count <= 0) {
    return [];
  }
  const order: number[] = [0];
  const seen = new Set(order);
  if (count > 1) {
    order.push(count - 1);
    seen.add(count - 1);
  }
  let gaps: Array<[number, number]> = [[0, count - 1]];
  while (gaps.length) {
    const next: Array<[number, number]> = [];
    for (const [low, high] of gaps) {
      if (high - low < 2) {
        continue;
      }
      const mid = Math.floor((low + high) / 2);
      if (!seen.has(mid)) {
        seen.add(mid);
        order.push(mid);
      }
      next.push([low, mid], [mid, high]);
    }
    gaps = next;
  }
  return order;
}

/** Index of the sample nearest to `sourceTime`. */
export function nearestThumbnailIndex(sourceTime: number, durationSeconds: number, count: number): number {
  if (count <= 0 || !(durationSeconds > 0)) {
    return -1;
  }
  const index = Math.floor((sourceTime / durationSeconds) * count);
  return Math.min(count - 1, Math.max(0, index));
}
