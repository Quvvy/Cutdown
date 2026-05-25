import { describe, expect, it } from 'vitest';
import {
  MAX_TRACK_SPLIT_RATIO,
  MIN_AUDIO_TRACK_HEIGHT,
  MIN_TRACK_SPLIT_RATIO,
  MIN_VIDEO_TRACK_HEIGHT,
  calculateTimelineTrackHeights,
  parseStoredTrackSplitRatio,
  timelineTrackGridRows,
  trackSplitRatioFromDrag,
  usableTimelineTrackHeight,
} from './timelineTrackSizing';

describe('timelineTrackSizing', () => {
  it('calculates normal pane heights from a split ratio', () => {
    expect(calculateTimelineTrackHeights(200, 0.55)).toEqual({
      videoHeight: 110,
      audioHeight: 90,
      splitRatio: 0.55,
    });
  });

  it('keeps video height above its minimum', () => {
    const heights = calculateTimelineTrackHeights(100, 0);

    expect(heights.videoHeight).toBe(MIN_VIDEO_TRACK_HEIGHT);
    expect(heights.audioHeight).toBe(100 - MIN_VIDEO_TRACK_HEIGHT);
    expect(heights.splitRatio).toBe(MIN_VIDEO_TRACK_HEIGHT / 100);
  });

  it('keeps audio height above its minimum', () => {
    const heights = calculateTimelineTrackHeights(100, 1);

    expect(heights.videoHeight).toBe(100 - MIN_AUDIO_TRACK_HEIGHT);
    expect(heights.audioHeight).toBe(MIN_AUDIO_TRACK_HEIGHT);
    expect(heights.splitRatio).toBe((100 - MIN_AUDIO_TRACK_HEIGHT) / 100);
  });

  it('never returns less than the combined minimum track height', () => {
    expect(usableTimelineTrackHeight(20)).toBe(MIN_VIDEO_TRACK_HEIGHT + MIN_AUDIO_TRACK_HEIGHT);
  });

  it('clamps drag resizing to supported split ratios', () => {
    expect(trackSplitRatioFromDrag(0.55, -1000, 200)).toBe(MIN_TRACK_SPLIT_RATIO);
    expect(trackSplitRatioFromDrag(0.55, 1000, 200)).toBe(MAX_TRACK_SPLIT_RATIO);
    expect(trackSplitRatioFromDrag(0.5, 25, 100)).toBe(0.75);
  });

  it('restores only valid stored split ratios', () => {
    expect(parseStoredTrackSplitRatio('0.64')).toBe(0.64);
    expect(parseStoredTrackSplitRatio('nope')).toBe(0.55);
    expect(parseStoredTrackSplitRatio('0.01')).toBe(MIN_TRACK_SPLIT_RATIO);
    expect(parseStoredTrackSplitRatio('0.99')).toBe(MAX_TRACK_SPLIT_RATIO);
  });

  it('builds stable grid rows for the timeline body', () => {
    expect(timelineTrackGridRows(0.55)).toBe('25px minmax(42px, 0.55fr) 6px minmax(38px, 0.45fr)');
  });
});
