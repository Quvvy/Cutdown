import {
  segmentIndexAtSourceTime,
  sequenceToSourceTime,
  sourceToSequenceTime,
  type TimelineMappingSegment,
} from './timelineMapping';

export const PLAYBACK_BOUNDARY_LEAD_SECONDS = 0.08;

export type SequencePlaybackCallbacks = {
  pauseVideo: (options?: { emit?: boolean }) => void;
  playVideo: () => void;
  seekVideo: (sourceTime: number) => void;
  updateCurrentTime: (sourceTime: number) => void;
};

export type SequencePlaybackContext = {
  getSegments: () => TimelineMappingSegment[];
  getSourceTime: () => number;
  isRangeLoop: () => boolean;
  getPlaybackRate: () => number;
  afterAdvance?: () => Promise<void>;
};

export type SequencePlaybackDriver = {
  dispose: () => void;
  reset: () => void;
  isPlaying: () => boolean;
  getSegmentIndex: () => number | null;
  syncFromSourceTime: (sourceTime: number) => void;
  onPlayState: (playing: boolean) => void;
  onTimeUpdate: (sourceTime: number) => void;
  onEnded: () => void;
  onUserSeek: (sourceTime: number) => void;
  onSegmentsChanged: () => void;
};

type DriverPhase = 'idle' | 'playing' | 'advancing' | 'stopped-at-end';

export function createSequencePlaybackDriver(
  callbacks: SequencePlaybackCallbacks,
  context: SequencePlaybackContext,
): SequencePlaybackDriver {
  let phase: DriverPhase = 'idle';
  let playing = false;
  let segmentIndex: number | null = null;
  let boundaryTimer: ReturnType<typeof setTimeout> | null = null;
  let awaitingSegmentIndex: number | null = null;
  let skipNextTimeupdate = false;
  let consumeNextEnded = false;

  function clearBoundary(): void {
    if (boundaryTimer) {
      clearTimeout(boundaryTimer);
      boundaryTimer = null;
    }
  }

  function resetTransitionGuards(): void {
    awaitingSegmentIndex = null;
    skipNextTimeupdate = false;
  }

  function reset(): void {
    clearBoundary();
    resetTransitionGuards();
    consumeNextEnded = false;
    phase = 'idle';
    playing = false;
    segmentIndex = null;
  }

  function syncFromSourceTime(sourceTime: number): void {
    segmentIndex = segmentIndexAtSourceTime(context.getSegments(), sourceTime);
  }

  function armBoundary(): void {
    clearBoundary();

    const segments = context.getSegments();
    if (context.isRangeLoop() || !playing || segments.length === 0) {
      return;
    }

    const index = segmentIndex ?? segmentIndexAtSourceTime(segments, context.getSourceTime());
    if (index === null) {
      return;
    }

    const segment = segments[index];
    if (!segment) {
      return;
    }

    segmentIndex = index;
    const sourceTime = context.getSourceTime();
    const withinSegment =
      sourceTime >= segment.sourceStart && sourceTime <= segment.sourceEnd;
    const referenceTime = withinSegment ? sourceTime : segment.sourceStart;
    const secondsUntilBoundary =
      (segment.sourceEnd - referenceTime - PLAYBACK_BOUNDARY_LEAD_SECONDS) /
      Math.max(0.25, context.getPlaybackRate());

    boundaryTimer = setTimeout(() => {
      void advanceToNextSegment();
    }, Math.max(0, secondsUntilBoundary * 1000));
  }

  async function advanceToNextSegment(forceResume = false): Promise<void> {
    clearBoundary();

    if (phase === 'advancing') {
      return;
    }

    const segments = context.getSegments();
    if (context.isRangeLoop() || (!playing && !forceResume) || segments.length === 0) {
      return;
    }

    phase = 'advancing';

    const index = segmentIndex ?? segmentIndexAtSourceTime(segments, context.getSourceTime());
    if (index === null) {
      playing = false;
      phase = 'idle';
      callbacks.pauseVideo();
      return;
    }

    const segment = segments[index];
    const nextSegment = segments[index + 1];
    const shouldResume = playing || forceResume;

    if (!nextSegment) {
      playing = false;
      phase = 'stopped-at-end';
      consumeNextEnded = true;
      callbacks.pauseVideo();
      segmentIndex = index;
      const endTime = Math.max(segment.sourceStart, segment.sourceEnd - 0.001);
      callbacks.updateCurrentTime(endTime);
      resetTransitionGuards();
      return;
    }

    callbacks.pauseVideo({ emit: false });
    segmentIndex = index + 1;
    awaitingSegmentIndex = index + 1;
    skipNextTimeupdate = true;
    callbacks.seekVideo(nextSegment.sourceStart);
    callbacks.updateCurrentTime(nextSegment.sourceStart);

    if (context.afterAdvance) {
      await context.afterAdvance();
    }

    // Keep awaitingSegmentIndex set until onTimeUpdate confirms the seek landed.
    phase = shouldResume ? 'playing' : 'idle';

    if (shouldResume) {
      playing = true;
      callbacks.playVideo();
      armBoundary();
    }
  }

  function resolvePlayheadForGap(): void {
    const segments = context.getSegments();
    if (segments.length === 0) {
      return;
    }

    let index = segmentIndexAtSourceTime(segments, context.getSourceTime());
    if (index !== null) {
      segmentIndex = index;
      return;
    }

    const sequenceTime = sourceToSequenceTime(segments, context.getSourceTime());
    const sourceTime = sequenceToSourceTime(segments, sequenceTime);
    index = segmentIndexAtSourceTime(segments, sourceTime);
    segmentIndex = index;
    callbacks.seekVideo(sourceTime);
    callbacks.updateCurrentTime(sourceTime);
  }

  function onPlayState(nextPlaying: boolean): void {
    playing = nextPlaying;

    if (!nextPlaying) {
      phase = phase === 'stopped-at-end' ? 'stopped-at-end' : 'idle';
      resetTransitionGuards();
      clearBoundary();
      return;
    }

    consumeNextEnded = false;
    if (phase === 'stopped-at-end') {
      phase = 'idle';
    }

    if (context.isRangeLoop() || context.getSegments().length === 0) {
      return;
    }

    resolvePlayheadForGap();
    armBoundary();
  }

  function onTimeUpdate(sourceTime: number): void {
    if (phase === 'advancing') {
      return;
    }

    if (skipNextTimeupdate) {
      skipNextTimeupdate = false;
      return;
    }

    if (awaitingSegmentIndex !== null) {
      const segments = context.getSegments();
      const expected = segments[awaitingSegmentIndex];
      const withinExpected =
        expected !== undefined &&
        sourceTime >= expected.sourceStart &&
        sourceTime < expected.sourceEnd;

      if (!withinExpected) {
        return;
      }

      resetTransitionGuards();
    }

    if (!context.isRangeLoop() && context.getSegments().length > 0) {
      const index = segmentIndexAtSourceTime(context.getSegments(), sourceTime);
      if (index !== null) {
        segmentIndex = index;
      }
    }

    callbacks.updateCurrentTime(sourceTime);
  }

  function onEnded(): void {
    if (context.isRangeLoop()) {
      return;
    }

    if (phase === 'advancing') {
      return;
    }

    if (consumeNextEnded) {
      consumeNextEnded = false;
      return;
    }

    const segments = context.getSegments();
    if (segmentIndex !== null) {
      const segment = segments[segmentIndex];
      if (segment && Math.abs(context.getSourceTime() - segment.sourceEnd) > 0.5) {
        return;
      }
    }

    void advanceToNextSegment(true);
  }

  function onUserSeek(sourceTime: number): void {
    consumeNextEnded = false;
    resetTransitionGuards();
    if (phase === 'stopped-at-end') {
      phase = 'idle';
    }
    syncFromSourceTime(sourceTime);
    if (playing) {
      armBoundary();
    }
  }

  function onSegmentsChanged(): void {
    syncFromSourceTime(context.getSourceTime());
    if (playing) {
      armBoundary();
    }
  }

  return {
    dispose: reset,
    reset,
    isPlaying: () => playing,
    getSegmentIndex: () => segmentIndex,
    syncFromSourceTime,
    onPlayState,
    onTimeUpdate,
    onEnded,
    onUserSeek,
    onSegmentsChanged,
  };
}
