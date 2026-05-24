import type { TimelineSegment } from '../stores/editor';

export const MIN_SEGMENT_SECONDS = 0.05;

export type SegmentEdge = 'start' | 'end';

function intervalsOverlap(startA: number, endA: number, startB: number, endB: number): boolean {
  return startA < endB && startB < endA;
}

export function clampSegmentEdge(
  segment: TimelineSegment,
  edge: SegmentEdge,
  proposed: number,
  duration: number,
  others: TimelineSegment[],
): number {
  if (edge === 'start') {
    return clampSegmentStart(segment, proposed, others);
  }

  return clampSegmentEnd(segment, proposed, duration, others);
}

export function resizeSegmentBounds(
  segment: TimelineSegment,
  edge: SegmentEdge,
  proposed: number,
  duration: number,
  allSegments: TimelineSegment[],
): { sourceStart: number; sourceEnd: number } {
  const others = allSegments.filter((candidate) => candidate.id !== segment.id);

  if (edge === 'start') {
    return {
      sourceStart: clampSegmentStart(segment, proposed, others),
      sourceEnd: segment.sourceEnd,
    };
  }

  return {
    sourceStart: segment.sourceStart,
    sourceEnd: clampSegmentEnd(segment, proposed, duration, others),
  };
}

function clampSegmentStart(
  segment: TimelineSegment,
  proposed: number,
  others: TimelineSegment[],
): number {
  const fixedEnd = segment.sourceEnd;
  const minStart = 0;
  const maxStart = fixedEnd - MIN_SEGMENT_SECONDS;
  let nextStart = clamp(proposed, minStart, maxStart);

  for (const other of others) {
    if (intervalsOverlap(nextStart, fixedEnd, other.sourceStart, other.sourceEnd)) {
      nextStart = Math.max(nextStart, other.sourceEnd);
    }
  }

  return clamp(nextStart, minStart, maxStart);
}

function clampSegmentEnd(
  segment: TimelineSegment,
  proposed: number,
  duration: number,
  others: TimelineSegment[],
): number {
  const fixedStart = segment.sourceStart;
  const minEnd = fixedStart + MIN_SEGMENT_SECONDS;
  const maxEnd = duration;
  let nextEnd = clamp(proposed, minEnd, maxEnd);

  for (const other of others) {
    if (intervalsOverlap(fixedStart, nextEnd, other.sourceStart, other.sourceEnd)) {
      nextEnd = Math.min(nextEnd, other.sourceStart);
    }
  }

  return clamp(nextEnd, minEnd, maxEnd);
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
