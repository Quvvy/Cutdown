import { describe, expect, it } from 'vitest';
import { coarseToFineOrder, nearestThumbnailIndex, thumbnailCount, thumbnailTimes } from './thumbnailPlan';

describe('thumbnailPlan', () => {
  it('scales the count with length, within bounds', () => {
    expect(thumbnailCount(0)).toBe(0);
    expect(thumbnailCount(3)).toBe(12);
    expect(thumbnailCount(12.5)).toBe(25);
    expect(thumbnailCount(7200)).toBe(60);
  });

  it('samples the middle of each slice', () => {
    expect(thumbnailTimes(10, 4)).toEqual([1.25, 3.75, 6.25, 8.75]);
  });

  it('orders coarse to fine and covers every index once', () => {
    const order = coarseToFineOrder(9);
    expect(order.slice(0, 3)).toEqual([0, 8, 4]);
    expect([...order].sort((a, b) => a - b)).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
    for (const count of [1, 2, 3, 25, 60]) {
      expect(new Set(coarseToFineOrder(count)).size).toBe(count);
    }
  });

  it('maps a source time to its slice', () => {
    expect(nearestThumbnailIndex(0, 10, 4)).toBe(0);
    expect(nearestThumbnailIndex(6, 10, 4)).toBe(2);
    expect(nearestThumbnailIndex(10, 10, 4)).toBe(3);
  });
});
