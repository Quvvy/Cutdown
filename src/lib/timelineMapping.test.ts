import { describe, expect, it } from 'vitest';
import {
  outputDurationForSegments,
  sequenceRangeForSegment,
  sequenceToSourceTime,
  segmentIndexAtSequenceTime,
  segmentIndexAtSourceTime,
  sourceToSequenceTime,
} from './timelineMapping';

const segments = [
  { id: 'a', sourceStart: 10, sourceEnd: 20 },
  { id: 'b', sourceStart: 40, sourceEnd: 45 },
  { id: 'c', sourceStart: 30, sourceEnd: 33 },
];

describe('timelineMapping', () => {
  it('calculates output duration from source ranges', () => {
    expect(outputDurationForSegments(segments)).toBe(18);
  });

  it('maps sequence time into reordered source segments', () => {
    expect(sequenceToSourceTime(segments, 0)).toBe(10);
    expect(sequenceToSourceTime(segments, 12)).toBe(42);
    expect(sequenceToSourceTime(segments, 18)).toBe(33);
  });

  it('maps source time to nearest sequence position', () => {
    expect(sourceToSequenceTime(segments, 42)).toBe(12);
    expect(sourceToSequenceTime(segments, 35)).toBe(18);
    expect(sourceToSequenceTime(segments, 21)).toBe(10);
  });

  it('returns sequence ranges for selected segments', () => {
    expect(sequenceRangeForSegment(segments, 'b')).toEqual({ start: 10, end: 15 });
    expect(sequenceRangeForSegment(segments, 'missing')).toBeNull();
  });

  it('finds timeline segment positions in reordered sequences', () => {
    expect(segmentIndexAtSequenceTime(segments, 0)).toBe(0);
    expect(segmentIndexAtSequenceTime(segments, 12)).toBe(1);
    expect(segmentIndexAtSequenceTime(segments, 16)).toBe(2);
    expect(segmentIndexAtSequenceTime(segments, 19)).toBeNull();
  });

  it('finds the source segment for a concrete preview time', () => {
    expect(segmentIndexAtSourceTime(segments, 15)).toBe(0);
    expect(segmentIndexAtSourceTime(segments, 42)).toBe(1);
    expect(segmentIndexAtSourceTime(segments, 31)).toBe(2);
    expect(segmentIndexAtSourceTime(segments, 25)).toBeNull();
  });
});
