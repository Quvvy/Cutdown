import { describe, expect, it } from 'vitest';
import { compactTimestamp, quickExportPath, quickExportRanges, remainingLabel, totalSeconds } from './quickExport';

const seg = (sourceStart: number, sourceEnd: number) => ({ sourceStart, sourceEnd });

describe('quickExportRanges', () => {
  it('exports all kept segments when no range is set', () => {
    expect(quickExportRanges([seg(0, 4), seg(7, 12.5)], null)).toEqual([
      { start: 0, end: 4 },
      { start: 7, end: 12.5 },
    ]);
  });

  it('trims kept segments to In/Out, so a cut-out middle stays out', () => {
    const parts = quickExportRanges([seg(0, 4), seg(7, 12.5)], { start: 2, end: 9 });
    expect(parts).toEqual([
      { start: 2, end: 4 },
      { start: 7, end: 9 },
    ]);
    expect(totalSeconds(parts)).toBe(4);
  });

  it('drops segments outside the range and slivers', () => {
    expect(quickExportRanges([seg(0, 4), seg(7, 12.5)], { start: 4.02, end: 6 })).toEqual([]);
  });

  it('merges back-to-back parts left by a cut with nothing removed', () => {
    expect(quickExportRanges([seg(0, 4), seg(4, 6), seg(6, 12.5)], { start: 2, end: 9 })).toEqual([{ start: 2, end: 9 }]);
  });

  it('keeps timeline order for reordered segments', () => {
    expect(quickExportRanges([seg(7, 10), seg(0, 3)], null)).toEqual([
      { start: 7, end: 10 },
      { start: 0, end: 3 },
    ]);
  });
});

describe('quickExportPath', () => {
  it('saves next to the original with the start time', () => {
    expect(quickExportPath('C:\\Videos\\Replay 2026-10-26.mp4', 14.7, 'mp4')).toBe('C:\\Videos\\Replay 2026-10-26-0m14s.mp4');
  });

  it('numbers repeats and handles long clips', () => {
    expect(quickExportPath('/v/clip.mkv', 3725, 'mp4', 2)).toBe('/v/clip-1h02m05s (2).mp4');
    expect(compactTimestamp(750)).toBe('12m30s');
  });
});

describe('remainingLabel', () => {
  it('waits for enough progress, then estimates', () => {
    expect(remainingLabel(0.5, 10)).toBeNull();
    expect(remainingLabel(2, 1)).toBeNull();
    expect(remainingLabel(3, 50)).toBe('about 3 s left');
    expect(remainingLabel(60, 25)).toBe('about 3 min left');
    expect(remainingLabel(10, 95)).toBe('almost done');
  });
});
