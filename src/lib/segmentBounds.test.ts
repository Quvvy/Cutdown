import { describe, expect, it } from 'vitest';
import { clampSegmentEdge, MIN_SEGMENT_SECONDS, resizeSegmentBounds } from './segmentBounds';
import type { TimelineSegment } from '../stores/editor';

const segmentA: TimelineSegment = { id: 'a', sourceStart: 10, sourceEnd: 20 };
const segmentB: TimelineSegment = { id: 'b', sourceStart: 40, sourceEnd: 45 };
const segmentC: TimelineSegment = { id: 'c', sourceStart: 30, sourceEnd: 33 };

describe('segmentBounds', () => {
  it('extends start into a gap before the segment', () => {
    expect(clampSegmentEdge(segmentA, 'start', 5, 60, [segmentB, segmentC])).toBe(5);
  });

  it('extends end into a gap after the segment', () => {
    expect(clampSegmentEdge(segmentA, 'end', 25, 60, [segmentB, segmentC])).toBe(25);
  });

  it('shrinks start and end while keeping minimum duration', () => {
    expect(clampSegmentEdge(segmentA, 'start', 19.96, 60, [])).toBe(19.95);
    expect(clampSegmentEdge(segmentA, 'end', 10.04, 60, [])).toBe(10.05);
  });

  it('clamps to file bounds', () => {
    expect(clampSegmentEdge(segmentA, 'start', -5, 60, [])).toBe(0);
    expect(clampSegmentEdge(segmentA, 'end', 100, 60, [])).toBe(60);
  });

  it('blocks start extension into another segment', () => {
    const leftNeighbor: TimelineSegment = { id: 'left', sourceStart: 0, sourceEnd: 12 };
    expect(clampSegmentEdge(segmentA, 'start', 8, 60, [leftNeighbor])).toBe(12);
  });

  it('blocks end extension into another segment', () => {
    const rightNeighbor: TimelineSegment = { id: 'right', sourceStart: 18, sourceEnd: 28 };
    expect(clampSegmentEdge(segmentA, 'end', 24, 60, [rightNeighbor])).toBe(18);
  });

  it('returns updated bounds for edge resize', () => {
    expect(resizeSegmentBounds(segmentA, 'start', 7, 60, [segmentA, segmentB])).toEqual({
      sourceStart: 7,
      sourceEnd: 20,
    });
    expect(resizeSegmentBounds(segmentA, 'end', 24, 60, [segmentA, segmentB])).toEqual({
      sourceStart: 10,
      sourceEnd: 24,
    });
  });

  it('enforces minimum segment duration', () => {
    const resized = resizeSegmentBounds(segmentA, 'start', 19.99, 60, [segmentA]);
    expect(resized.sourceEnd - resized.sourceStart).toBeGreaterThanOrEqual(MIN_SEGMENT_SECONDS);
  });
});
