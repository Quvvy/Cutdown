import { writable } from 'svelte/store';

export type VideoMetadata = {
  duration: number;
  fps: number;
  codec: string;
  width: number;
  height: number;
  fileSize: number;
  audioCodec: string | null;
  audioChannels: number | null;
};

export type ExportStatus = {
  state: 'idle' | 'running' | 'success' | 'error';
  message: string;
  outputPath?: string;
  outputSize?: number;
};

export type TimelineSegment = {
  id: string;
  sourceStart: number;
  sourceEnd: number;
};

export type EditorState = {
  currentFile: string | null;
  videoSrc: string | null;
  previewTempPath: string | null;
  previewStrategy: 'Native preview' | 'Preview remux' | 'Preview proxy';
  metadata: VideoMetadata | null;
  currentTime: number;
  segments: TimelineSegment[];
  selectedSegmentId: string | null;
  exportStatus: ExportStatus;
};

const initialState: EditorState = {
  currentFile: null,
  videoSrc: null,
  previewTempPath: null,
  previewStrategy: 'Native preview',
  metadata: null,
  currentTime: 0,
  segments: [],
  selectedSegmentId: null,
  exportStatus: {
    state: 'idle',
    message: 'Choose a clip to begin.',
  },
};

export const editor = writable<EditorState>(initialState);

export function createFullSegment(duration: number): TimelineSegment {
  return {
    id: crypto.randomUUID(),
    sourceStart: 0,
    sourceEnd: duration,
  };
}

export function segmentDuration(segment: TimelineSegment): number {
  return Math.max(0, segment.sourceEnd - segment.sourceStart);
}

export function totalSegmentDuration(segments: TimelineSegment[]): number {
  return segments.reduce((total, segment) => total + segmentDuration(segment), 0);
}

export function sortSegments(segments: TimelineSegment[]): TimelineSegment[] {
  return [...segments].sort((a, b) => a.sourceStart - b.sourceStart);
}

export function resetEditor(): void {
  editor.set(initialState);
}
