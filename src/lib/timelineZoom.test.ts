import { describe, expect, it } from 'vitest';
import { MAX_ZOOM, pixelsPerSecondForZoom, zoomForPixelsPerSecond, zoomToFit } from './timelineZoom';

describe('timelineZoom', () => {
  it('round-trips between zoom and pixels per second', () => {
    for (const zoom of [0, 40, 160, 225, MAX_ZOOM]) {
      expect(zoomForPixelsPerSecond(pixelsPerSecondForZoom(zoom))).toBeCloseTo(zoom, 6);
    }
  });

  it('fits a 2-hour recording and a 3-second moment', () => {
    const viewport = 1200;
    expect(7200 * pixelsPerSecondForZoom(zoomToFit(7200, viewport))).toBeCloseTo(viewport, 3);
    expect(3 * pixelsPerSecondForZoom(zoomToFit(3, viewport))).toBeCloseTo(136 * 3, 3);
    expect(12.5 * pixelsPerSecondForZoom(zoomToFit(12.5, viewport))).toBeCloseTo(viewport, 3);
  });

  it('zooms by the same ratio per step anywhere on the slider', () => {
    const ratio = (zoom: number) => pixelsPerSecondForZoom(zoom + 6) / pixelsPerSecondForZoom(zoom);
    expect(ratio(10)).toBeCloseTo(ratio(250), 9);
  });
});
