import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createSequencePlaybackDriver, PLAYBACK_BOUNDARY_LEAD_SECONDS } from './sequencePlayback';

const segments = [
  { id: 'a', sourceStart: 10, sourceEnd: 20 },
  { id: 'b', sourceStart: 40, sourceEnd: 45 },
  { id: 'c', sourceStart: 30, sourceEnd: 33 },
];

describe('sequencePlayback', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('advances to the next timeline segment at the boundary', async () => {
    let currentTime = 10;
    const pauseVideo = vi.fn();
    const playVideo = vi.fn();
    const seekVideo = vi.fn((time: number) => {
      currentTime = time;
    });
    const updateCurrentTime = vi.fn((time: number) => {
      currentTime = time;
    });

    const driver = createSequencePlaybackDriver(
      { pauseVideo, playVideo, seekVideo, updateCurrentTime },
      {
        getSegments: () => segments,
        getSourceTime: () => currentTime,
        isRangeLoop: () => false,
        getPlaybackRate: () => 1,
        afterAdvance: async () => undefined,
      },
    );

    driver.onPlayState(true);
    expect(driver.getSegmentIndex()).toBe(0);

    const firstBoundaryMs = (20 - 10 - PLAYBACK_BOUNDARY_LEAD_SECONDS) * 1000;
    await vi.advanceTimersByTimeAsync(firstBoundaryMs);

    expect(seekVideo).toHaveBeenCalledWith(40);
    expect(playVideo).toHaveBeenCalled();
    expect(driver.getSegmentIndex()).toBe(1);
    driver.dispose();
  });

  it('stops at the last segment without seeking the video', async () => {
    let currentTime = 30;
    const pauseVideo = vi.fn();
    const playVideo = vi.fn();
    const seekVideo = vi.fn();
    const updateCurrentTime = vi.fn((time: number) => {
      currentTime = time;
    });

    const driver = createSequencePlaybackDriver(
      { pauseVideo, playVideo, seekVideo, updateCurrentTime },
      {
        getSegments: () => segments,
        getSourceTime: () => currentTime,
        isRangeLoop: () => false,
        getPlaybackRate: () => 1,
        afterAdvance: async () => undefined,
      },
    );

    driver.syncFromSourceTime(currentTime);
    driver.onPlayState(true);
    expect(driver.getSegmentIndex()).toBe(2);

    const lastBoundaryMs = (33 - 30 - PLAYBACK_BOUNDARY_LEAD_SECONDS) * 1000;
    await vi.advanceTimersByTimeAsync(lastBoundaryMs);

    expect(pauseVideo).toHaveBeenCalled();
    expect(seekVideo).not.toHaveBeenCalled();
    expect(updateCurrentTime).toHaveBeenCalledWith(32.999);
    driver.dispose();
  });

  it('ignores stale timeupdates after a segment transition', async () => {
    let currentTime = 19.5;
    const updateCurrentTime = vi.fn((time: number) => {
      currentTime = time;
    });

    const driver = createSequencePlaybackDriver(
      {
        pauseVideo: vi.fn(),
        playVideo: vi.fn(),
        seekVideo: vi.fn((time: number) => {
          currentTime = time;
        }),
        updateCurrentTime,
      },
      {
        getSegments: () => segments,
        getSourceTime: () => currentTime,
        isRangeLoop: () => false,
        getPlaybackRate: () => 1,
        afterAdvance: async () => undefined,
      },
    );

    driver.onPlayState(true);
    updateCurrentTime.mockClear();

    const firstBoundaryMs = (20 - 19.5 - PLAYBACK_BOUNDARY_LEAD_SECONDS) * 1000;
    await vi.advanceTimersByTimeAsync(firstBoundaryMs);

    updateCurrentTime.mockClear();
    driver.onTimeUpdate(15);
    expect(updateCurrentTime).not.toHaveBeenCalled();

    driver.onTimeUpdate(40);
    expect(updateCurrentTime).toHaveBeenCalledWith(40);
    driver.dispose();
  });
});
