export type TimelineMappingSegment = {
  id: string;
  sourceStart: number;
  sourceEnd: number;
};

export function segmentDuration(segment: TimelineMappingSegment): number {
  return Math.max(0, segment.sourceEnd - segment.sourceStart);
}

export function outputDurationForSegments(segments: TimelineMappingSegment[]): number {
  return segments.reduce((total, segment) => total + segmentDuration(segment), 0);
}

export function sequenceRangeForSegment(
  segments: TimelineMappingSegment[],
  segmentId: string,
): { start: number; end: number } | null {
  let cursor = 0;

  for (const segment of segments) {
    const length = segmentDuration(segment);
    if (segment.id === segmentId) {
      return { start: cursor, end: cursor + length };
    }
    cursor += length;
  }

  return null;
}

export function sequenceToSourceTime(
  segments: TimelineMappingSegment[],
  sequenceTime: number,
): number {
  let cursor = 0;

  for (const segment of segments) {
    const length = segmentDuration(segment);

    if (sequenceTime <= cursor + length) {
      return segment.sourceStart + clamp(sequenceTime - cursor, 0, length);
    }

    cursor += length;
  }

  return segments[segments.length - 1]?.sourceEnd ?? 0;
}

export function sourceToSequenceTime(
  segments: TimelineMappingSegment[],
  sourceTime: number,
): number {
  let cursor = 0;
  let nearestSequenceTime = 0;
  let nearestDistance = Number.POSITIVE_INFINITY;

  for (const segment of segments) {
    const length = segmentDuration(segment);

    if (sourceTime >= segment.sourceStart && sourceTime <= segment.sourceEnd) {
      return cursor + clamp(sourceTime - segment.sourceStart, 0, length);
    }

    const startDistance = Math.abs(sourceTime - segment.sourceStart);
    if (startDistance < nearestDistance) {
      nearestDistance = startDistance;
      nearestSequenceTime = cursor;
    }

    const endDistance = Math.abs(sourceTime - segment.sourceEnd);
    if (endDistance < nearestDistance) {
      nearestDistance = endDistance;
      nearestSequenceTime = cursor + length;
    }

    cursor += length;
  }

  return clamp(nearestSequenceTime, 0, outputDurationForSegments(segments));
}

export function segmentIndexAtSourceTime(
  segments: TimelineMappingSegment[],
  sourceTime: number,
): number | null {
  for (let index = 0; index < segments.length; index += 1) {
    const segment = segments[index];
    const isLast = index === segments.length - 1;
    // Use [sourceStart, sourceEnd) for non-last segments so the boundary point
    // belongs to the NEXT segment, not the previous one (prevents aliasing when
    // seeking to exactly nextSegment.sourceStart === prevSegment.sourceEnd).
    // The final segment uses [sourceStart, sourceEnd] to cover its exact endpoint.
    if (
      sourceTime >= segment.sourceStart &&
      (isLast ? sourceTime <= segment.sourceEnd : sourceTime < segment.sourceEnd)
    ) {
      return index;
    }
  }

  return null;
}

export function segmentIndexAtSequenceTime(
  segments: TimelineMappingSegment[],
  sequenceTime: number,
): number | null {
  let cursor = 0;

  for (let index = 0; index < segments.length; index += 1) {
    const length = segmentDuration(segments[index]);
    if (sequenceTime >= cursor && sequenceTime <= cursor + length) {
      return index;
    }
    cursor += length;
  }

  return null;
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}
