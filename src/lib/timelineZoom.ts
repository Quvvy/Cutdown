// Timeline zoom slider ↔ pixels per second. Exponential, so one slider covers a
// 2-hour recording on screen and a 3-second moment filling it, and each wheel notch
// zooms by the same ratio at any level.

export const MAX_ZOOM = 320;
const MIN_PIXELS_PER_SECOND = 0.05;
const MAX_PIXELS_PER_SECOND = 136;
const RANGE = Math.log(MAX_PIXELS_PER_SECOND / MIN_PIXELS_PER_SECOND);

export function pixelsPerSecondForZoom(zoom: number): number {
  const clamped = Math.min(MAX_ZOOM, Math.max(0, zoom));
  return MIN_PIXELS_PER_SECOND * Math.exp((clamped / MAX_ZOOM) * RANGE);
}

export function zoomForPixelsPerSecond(pixelsPerSecond: number): number {
  if (!(pixelsPerSecond > 0)) {
    return 0;
  }
  const zoom = (MAX_ZOOM * Math.log(pixelsPerSecond / MIN_PIXELS_PER_SECOND)) / RANGE;
  return Math.min(MAX_ZOOM, Math.max(0, zoom));
}

/** Zoom that fits `seconds` into `viewportPixels`. */
export function zoomToFit(seconds: number, viewportPixels: number): number {
  if (!(seconds > 0) || !(viewportPixels > 0)) {
    return zoomForPixelsPerSecond(13);
  }
  return zoomForPixelsPerSecond(viewportPixels / seconds);
}
