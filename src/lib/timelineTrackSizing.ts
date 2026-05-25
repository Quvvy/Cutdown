export const TIMELINE_RULER_HEIGHT = 25;
export const TIMELINE_RESIZE_BAR_HEIGHT = 6;
export const MIN_VIDEO_TRACK_HEIGHT = 42;
export const MIN_AUDIO_TRACK_HEIGHT = 38;
export const DEFAULT_TRACK_SPLIT_RATIO = 0.55;
export const MIN_TRACK_SPLIT_RATIO = 0.22;
export const MAX_TRACK_SPLIT_RATIO = 0.78;

export type TimelineTrackHeights = {
  videoHeight: number;
  audioHeight: number;
  splitRatio: number;
};

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function formatRatio(value: number): string {
  return Number(value.toFixed(4)).toString();
}

export function normalizeTrackSplitRatio(
  value: number,
  fallback = DEFAULT_TRACK_SPLIT_RATIO,
): number {
  const safeFallback = clamp(fallback, MIN_TRACK_SPLIT_RATIO, MAX_TRACK_SPLIT_RATIO);
  return Number.isFinite(value) ? clamp(value, MIN_TRACK_SPLIT_RATIO, MAX_TRACK_SPLIT_RATIO) : safeFallback;
}

export function parseStoredTrackSplitRatio(raw: string | null): number {
  const parsed = raw ? Number.parseFloat(raw) : Number.NaN;
  return normalizeTrackSplitRatio(parsed);
}

export function usableTimelineTrackHeight(bodyHeight: number): number {
  return Math.max(
    MIN_VIDEO_TRACK_HEIGHT + MIN_AUDIO_TRACK_HEIGHT,
    bodyHeight - TIMELINE_RULER_HEIGHT - TIMELINE_RESIZE_BAR_HEIGHT,
  );
}

export function calculateTimelineTrackHeights(
  usableHeight: number,
  splitRatio: number,
): TimelineTrackHeights {
  const usable = Math.max(MIN_VIDEO_TRACK_HEIGHT + MIN_AUDIO_TRACK_HEIGHT, usableHeight);
  const normalizedSplit = normalizeTrackSplitRatio(splitRatio);
  const videoHeight = clamp(
    Math.round(usable * normalizedSplit),
    MIN_VIDEO_TRACK_HEIGHT,
    usable - MIN_AUDIO_TRACK_HEIGHT,
  );
  const audioHeight = Math.max(MIN_AUDIO_TRACK_HEIGHT, usable - videoHeight);

  return {
    videoHeight,
    audioHeight,
    splitRatio: videoHeight / usable,
  };
}

export function trackSplitRatioFromDrag(
  startRatio: number,
  deltaY: number,
  usableHeight: number,
): number {
  return normalizeTrackSplitRatio(startRatio + deltaY / Math.max(usableHeight, 1));
}

export function timelineTrackGridRows(splitRatio: number): string {
  const normalizedSplit = normalizeTrackSplitRatio(splitRatio);
  return `${TIMELINE_RULER_HEIGHT}px minmax(${MIN_VIDEO_TRACK_HEIGHT}px, ${formatRatio(normalizedSplit)}fr) ${TIMELINE_RESIZE_BAR_HEIGHT}px minmax(${MIN_AUDIO_TRACK_HEIGHT}px, ${formatRatio(1 - normalizedSplit)}fr)`;
}
