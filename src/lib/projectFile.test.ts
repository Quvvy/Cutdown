import { describe, expect, it } from 'vitest';
import { CUTDOWN_PROJECT_VERSION, createCutdownProjectPayload } from './projectFile';

describe('projectFile', () => {
  it('builds versioned project payloads without sharing mutable arrays', () => {
    const segments = [{ id: 'segment-1', sourceStart: 1, sourceEnd: 4 }];
    const bookmarks = [{ id: 'bookmark-1', time: 2, label: 'Nice cut' }];
    const cropRect = { x: 0.1, y: 0.2, width: 0.7, height: 0.6 };

    const payload = createCutdownProjectPayload({
      sourcePath: 'E:\\clips\\sample.mp4',
      segments,
      selectedSegmentId: 'segment-1',
      rangeStart: 1,
      rangeEnd: 4,
      cropEnabled: true,
      cropRect,
      clipVolume: 0.8,
      currentTime: 2,
      bookmarks,
      exportPresetId: 'lossless-trim',
      accurateTrim: false,
      stripAudio: false,
    });

    expect(payload.version).toBe(CUTDOWN_PROJECT_VERSION);
    expect(payload.segments).toEqual(segments);
    expect(payload.bookmarks).toEqual(bookmarks);
    expect(payload.cropRect).toEqual(cropRect);
    expect(payload.segments).not.toBe(segments);
    expect(payload.bookmarks).not.toBe(bookmarks);
    expect(payload.cropRect).not.toBe(cropRect);
  });
});
