import type { TimelineSegment } from '../stores/editor';
import type { TimelineBookmark } from './types';
import { clamp, formatTime } from './format';
import { MIN_SEGMENT_SECONDS } from './segmentBounds';

export type IdFactory = () => string;
export type TimelineRange = { start: number; end: number };

export type SegmentEditResult = {
  segments: TimelineSegment[];
  selectedSegmentId: string | null;
  changed: boolean;
};

export type DeleteSegmentResult = SegmentEditResult & {
  status: 'deleted' | 'missing' | 'minimum';
};

export type BookmarkEditResult = {
  bookmarks: TimelineBookmark[];
  selectedBookmarkId: string | null;
  changed: boolean;
};

export function createTimelineId(): string {
  return crypto.randomUUID();
}

export function createInitialTimelineSegments(duration: number, createId: IdFactory = createTimelineId): TimelineSegment[] {
  return [
    {
      id: createId(),
      sourceStart: 0,
      sourceEnd: duration,
    },
  ];
}

function isInsideSegment(segment: TimelineSegment, seconds: number): boolean {
  return seconds > segment.sourceStart + MIN_SEGMENT_SECONDS && seconds < segment.sourceEnd - MIN_SEGMENT_SECONDS;
}

export function splitTimelineSegments(
  segments: TimelineSegment[],
  splitTime: number,
  createId: IdFactory = createTimelineId,
): SegmentEditResult {
  if (!segments.some((segment) => isInsideSegment(segment, splitTime))) {
    return {
      segments,
      selectedSegmentId: null,
      changed: false,
    };
  }

  return {
    segments: segments.flatMap((segment) => {
      if (!isInsideSegment(segment, splitTime)) {
        return [segment];
      }

      return [
        {
          ...segment,
          sourceEnd: splitTime,
        },
        {
          id: createId(),
          sourceStart: splitTime,
          sourceEnd: segment.sourceEnd,
        },
      ];
    }),
    selectedSegmentId: null,
    changed: true,
  };
}

export function deleteTimelineSegment(
  segments: TimelineSegment[],
  selectedSegmentId: string | null,
): DeleteSegmentResult {
  if (!selectedSegmentId || !segments.some((segment) => segment.id === selectedSegmentId)) {
    return {
      segments,
      selectedSegmentId,
      changed: false,
      status: 'missing',
    };
  }

  if (segments.length <= 1) {
    return {
      segments,
      selectedSegmentId,
      changed: false,
      status: 'minimum',
    };
  }

  return {
    segments: segments.filter((segment) => segment.id !== selectedSegmentId),
    selectedSegmentId: null,
    changed: true,
    status: 'deleted',
  };
}

export function duplicateTimelineSegment(
  segments: TimelineSegment[],
  selectedSegmentId: string | null,
  createId: IdFactory = createTimelineId,
): SegmentEditResult {
  const selected = segments.find((segment) => segment.id === selectedSegmentId);
  if (!selected) {
    return {
      segments,
      selectedSegmentId,
      changed: false,
    };
  }

  const duplicate = {
    id: createId(),
    sourceStart: selected.sourceStart,
    sourceEnd: selected.sourceEnd,
  };
  const index = segments.findIndex((segment) => segment.id === selected.id);
  const nextSegments = [...segments];
  nextSegments.splice(index + 1, 0, duplicate);

  return {
    segments: nextSegments,
    selectedSegmentId: duplicate.id,
    changed: true,
  };
}

export function reorderTimelineSegment(
  segments: TimelineSegment[],
  id: string,
  toIndex: number,
): SegmentEditResult {
  const fromIndex = segments.findIndex((segment) => segment.id === id);
  if (fromIndex < 0) {
    return {
      segments,
      selectedSegmentId: id,
      changed: false,
    };
  }

  const nextSegments = [...segments];
  const [moved] = nextSegments.splice(fromIndex, 1);
  const targetIndex = clamp(toIndex, 0, nextSegments.length);
  nextSegments.splice(targetIndex, 0, moved);

  return {
    segments: nextSegments,
    selectedSegmentId: id,
    changed: true,
  };
}

export function keepOnlyTimelineRange(range: TimelineRange, createId: IdFactory = createTimelineId): SegmentEditResult {
  return {
    segments: [
      {
        id: createId(),
        sourceStart: range.start,
        sourceEnd: range.end,
      },
    ],
    selectedSegmentId: null,
    changed: true,
  };
}

export function deleteOutsideTimelineRange(
  segments: TimelineSegment[],
  range: TimelineRange,
  createId: IdFactory = createTimelineId,
): SegmentEditResult {
  const trimmedSegments = segments.flatMap((segment) => {
    const overlapStart = Math.max(segment.sourceStart, range.start);
    const overlapEnd = Math.min(segment.sourceEnd, range.end);

    if (overlapEnd <= overlapStart + MIN_SEGMENT_SECONDS) {
      return [];
    }

    return [
      {
        ...segment,
        sourceStart: overlapStart,
        sourceEnd: overlapEnd,
      },
    ];
  });

  return {
    segments:
      trimmedSegments.length > 0
        ? trimmedSegments
        : [
            {
              id: createId(),
              sourceStart: range.start,
              sourceEnd: range.end,
            },
          ],
    selectedSegmentId: null,
    changed: true,
  };
}

export function setTimelineRangeMarker(
  marker: 'start' | 'end',
  seconds: number,
  duration: number,
  currentRange: { rangeStart: number | null; rangeEnd: number | null },
): { rangeStart: number | null; rangeEnd: number | null } {
  const nextTime = clamp(seconds, 0, duration);

  if (marker === 'start') {
    return {
      rangeStart: nextTime,
      rangeEnd: currentRange.rangeEnd ?? duration,
    };
  }

  return {
    rangeStart: currentRange.rangeStart ?? 0,
    rangeEnd: nextTime,
  };
}

export function clearTimelineRange(): { rangeStart: null; rangeEnd: null } {
  return { rangeStart: null, rangeEnd: null };
}

export function addTimelineBookmark(
  bookmarks: TimelineBookmark[],
  seconds: number,
  duration: number,
  options: {
    label?: string;
    defaultLabel: string;
    dedupeSeconds: number;
    createId?: IdFactory;
  },
): BookmarkEditResult & { status: 'added' | 'duplicate' } {
  const time = clamp(seconds, 0, duration);
  if (bookmarks.some((bookmark) => Math.abs(bookmark.time - time) < options.dedupeSeconds)) {
    return {
      bookmarks,
      selectedBookmarkId: null,
      changed: false,
      status: 'duplicate',
    };
  }

  const entry: TimelineBookmark = {
    id: (options.createId ?? createTimelineId)(),
    time,
    label: options.label?.trim() || options.defaultLabel,
  };

  return {
    bookmarks: [...bookmarks, entry].sort((left, right) => left.time - right.time),
    selectedBookmarkId: entry.id,
    changed: true,
    status: 'added',
  };
}

export function updateTimelineBookmarkLabel(
  bookmarks: TimelineBookmark[],
  id: string,
  label: string,
): TimelineBookmark[] {
  const trimmed = label.trim();
  return bookmarks.map((bookmark) =>
    bookmark.id === id ? { ...bookmark, label: trimmed || formatTime(bookmark.time) } : bookmark,
  );
}

export function removeTimelineBookmark(
  bookmarks: TimelineBookmark[],
  id: string,
  selectedBookmarkId: string | null,
): BookmarkEditResult {
  return {
    bookmarks: bookmarks.filter((bookmark) => bookmark.id !== id),
    selectedBookmarkId: selectedBookmarkId === id ? null : selectedBookmarkId,
    changed: bookmarks.some((bookmark) => bookmark.id === id),
  };
}
