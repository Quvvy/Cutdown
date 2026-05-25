import { describe, expect, it, vi } from 'vitest';
import {
  addTimelineBookmark,
  clearTimelineRange,
  createInitialTimelineSegments,
  deleteOutsideTimelineRange,
  deleteTimelineSegment,
  duplicateTimelineSegment,
  keepOnlyTimelineRange,
  removeTimelineBookmark,
  reorderTimelineSegment,
  setTimelineRangeMarker,
  splitTimelineSegments,
  updateTimelineBookmarkLabel,
} from './timelineEditing';

const ids = (prefix = 'id') => {
  let next = 1;
  return () => `${prefix}-${next++}`;
};

const segments = [
  { id: 'a', sourceStart: 0, sourceEnd: 10 },
  { id: 'b', sourceStart: 20, sourceEnd: 30 },
  { id: 'c', sourceStart: 40, sourceEnd: 50 },
];

describe('timelineEditing', () => {
  it('creates a fresh full-length segment for a raw video', () => {
    expect(createInitialTimelineSegments(42, ids('segment'))).toEqual([
      { id: 'segment-1', sourceStart: 0, sourceEnd: 42 },
    ]);
  });

  it('uses native randomUUID without detaching it from crypto', () => {
    type CryptoMock = { randomUUID: () => string };
    let fakeCrypto: CryptoMock;
    fakeCrypto = {
      randomUUID: vi.fn(function (this: CryptoMock) {
        if (this !== fakeCrypto) {
          throw new TypeError('Illegal invocation');
        }
        return 'native-bound-id';
      }),
    };
    vi.stubGlobal('crypto', fakeCrypto);

    try {
      expect(createInitialTimelineSegments(42)).toEqual([
        { id: 'native-bound-id', sourceStart: 0, sourceEnd: 42 },
      ]);
    } finally {
      vi.unstubAllGlobals();
    }
  });

  it('splits inside a segment', () => {
    expect(splitTimelineSegments([segments[0]], 4, ids('split'))).toEqual({
      segments: [
        { id: 'a', sourceStart: 0, sourceEnd: 4 },
        { id: 'split-1', sourceStart: 4, sourceEnd: 10 },
      ],
      selectedSegmentId: null,
      changed: true,
    });
  });

  it('leaves invalid splits unchanged', () => {
    const result = splitTimelineSegments([segments[0]], 0.02, ids('split'));

    expect(result.changed).toBe(false);
    expect(result.segments).toEqual([segments[0]]);
  });

  it('deletes the selected segment but keeps at least one segment', () => {
    expect(deleteTimelineSegment(segments, 'b')).toMatchObject({
      segments: [segments[0], segments[2]],
      selectedSegmentId: null,
      changed: true,
      status: 'deleted',
    });
    expect(deleteTimelineSegment([segments[0]], 'a')).toMatchObject({
      segments: [segments[0]],
      selectedSegmentId: 'a',
      changed: false,
      status: 'minimum',
    });
  });

  it('duplicates the selected segment after itself', () => {
    expect(duplicateTimelineSegment(segments, 'b', ids('copy'))).toEqual({
      segments: [
        segments[0],
        segments[1],
        { id: 'copy-1', sourceStart: 20, sourceEnd: 30 },
        segments[2],
      ],
      selectedSegmentId: 'copy-1',
      changed: true,
    });
  });

  it('reorders a segment while preserving identity', () => {
    expect(reorderTimelineSegment(segments, 'a', 2)).toEqual({
      segments: [segments[1], segments[2], segments[0]],
      selectedSegmentId: 'a',
      changed: true,
    });
  });

  it('keeps only an I/O range', () => {
    expect(keepOnlyTimelineRange({ start: 3, end: 7 }, ids('range'))).toEqual({
      segments: [{ id: 'range-1', sourceStart: 3, sourceEnd: 7 }],
      selectedSegmentId: null,
      changed: true,
    });
  });

  it('deletes footage outside an I/O range', () => {
    expect(deleteOutsideTimelineRange(segments, { start: 5, end: 25 }, ids('range'))).toEqual({
      segments: [
        { id: 'a', sourceStart: 5, sourceEnd: 10 },
        { id: 'b', sourceStart: 20, sourceEnd: 25 },
      ],
      selectedSegmentId: null,
      changed: true,
    });
    expect(deleteOutsideTimelineRange([segments[2]], { start: 5, end: 10 }, ids('range'))).toEqual({
      segments: [{ id: 'range-1', sourceStart: 5, sourceEnd: 10 }],
      selectedSegmentId: null,
      changed: true,
    });
  });

  it('sets and clears I/O range markers', () => {
    expect(setTimelineRangeMarker('start', -5, 60, { rangeStart: null, rangeEnd: null })).toEqual({
      rangeStart: 0,
      rangeEnd: 60,
    });
    expect(setTimelineRangeMarker('end', 80, 60, { rangeStart: null, rangeEnd: null })).toEqual({
      rangeStart: 0,
      rangeEnd: 60,
    });
    expect(clearTimelineRange()).toEqual({ rangeStart: null, rangeEnd: null });
  });

  it('adds, dedupes, renames, and removes bookmarks', () => {
    const added = addTimelineBookmark([], 4, 10, {
      defaultLabel: 'Marker 1',
      dedupeSeconds: 0.05,
      createId: ids('bookmark'),
    });

    expect(added).toMatchObject({
      bookmarks: [{ id: 'bookmark-1', time: 4, label: 'Marker 1' }],
      selectedBookmarkId: 'bookmark-1',
      changed: true,
      status: 'added',
    });

    expect(
      addTimelineBookmark(added.bookmarks, 4.02, 10, {
        defaultLabel: 'Marker 2',
        dedupeSeconds: 0.05,
        createId: ids('bookmark'),
      }),
    ).toMatchObject({
      bookmarks: added.bookmarks,
      selectedBookmarkId: null,
      changed: false,
      status: 'duplicate',
    });

    const renamed = updateTimelineBookmarkLabel(added.bookmarks, 'bookmark-1', ' ');
    expect(renamed[0].label).toBe('00:00:04.0');

    expect(removeTimelineBookmark(renamed, 'bookmark-1', 'bookmark-1')).toEqual({
      bookmarks: [],
      selectedBookmarkId: null,
      changed: true,
    });
  });
});
