import { describe, expect, it } from 'vitest';
import { computeFitScale, shouldAutoFitOnViewportResize } from './previewView';

describe('computeFitScale', () => {
  it('fits a 16:9 video into a wider viewport using height', () => {
    expect(computeFitScale(1920, 1080, 1600, 600)).toBeCloseTo(600 / 1080);
  });

  it('fits a 16:9 video into a taller viewport using width', () => {
    expect(computeFitScale(1920, 1080, 800, 900)).toBeCloseTo(800 / 1920);
  });

  it('returns 1 until both video and viewport have a size', () => {
    expect(computeFitScale(0, 1080, 800, 600)).toBe(1);
    expect(computeFitScale(1920, 1080, 0, 600)).toBe(1);
  });
});

describe('shouldAutoFitOnViewportResize', () => {
  it('auto-fits when the user has not zoomed or panned away from fit', () => {
    expect(shouldAutoFitOnViewportResize(false, 1920)).toBe(true);
  });

  it('does not auto-fit after the user zooms or pans', () => {
    expect(shouldAutoFitOnViewportResize(true, 1920)).toBe(false);
  });

  it('does not auto-fit before video dimensions are known', () => {
    expect(shouldAutoFitOnViewportResize(false, 0)).toBe(false);
  });
});
