import { beforeEach, describe, expect, it, vi } from 'vitest';
import { buildPerSegmentJobs } from './exportQueue';

describe('exportQueue', () => {
  beforeEach(() => {
    let counter = 0;
    vi.spyOn(crypto, 'randomUUID').mockImplementation(() => `job-${++counter}` as `${string}-${string}-${string}-${string}-${string}`);
  });

  it('builds one export job per segment with stable labels and paths', () => {
    const jobs = buildPerSegmentJobs('C:\\Clips', 'match.mp4', [
      { start: 0, end: 10 },
      { start: 25, end: 35 },
    ]);

    expect(jobs).toEqual([
      {
        id: 'job-1',
        outputPath: 'C:\\Clips\\match-seg01.mp4',
        segments: [{ start: 0, end: 10 }],
        label: 'Segment 1',
      },
      {
        id: 'job-2',
        outputPath: 'C:\\Clips\\match-seg02.mp4',
        segments: [{ start: 25, end: 35 }],
        label: 'Segment 2',
      },
    ]);
  });
});
