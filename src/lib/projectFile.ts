import type { TimelineSegment } from '../stores/editor';
import type { NormalizedCropRect, TimelineBookmark } from './types';

export const CUTDOWN_PROJECT_VERSION = 1;
export const CUTDOWN_PROJECT_EXTENSION = 'cutdown';

export type CutdownProject = {
  version: number;
  sourcePath: string;
  segments: TimelineSegment[];
  selectedSegmentId: string | null;
  rangeStart: number | null;
  rangeEnd: number | null;
  cropEnabled: boolean;
  cropRect: NormalizedCropRect;
  clipVolume: number;
  currentTime: number | null;
  bookmarks: TimelineBookmark[];
  exportPresetId: string | null;
  accurateTrim: boolean;
  stripAudio: boolean;
};

export type CutdownProjectDraft = Omit<CutdownProject, 'version'>;

export function createCutdownProjectPayload(draft: CutdownProjectDraft): CutdownProject {
  return {
    version: CUTDOWN_PROJECT_VERSION,
    ...draft,
    segments: draft.segments.map((segment) => ({ ...segment })),
    cropRect: { ...draft.cropRect },
    bookmarks: draft.bookmarks.map((bookmark) => ({ ...bookmark })),
  };
}
