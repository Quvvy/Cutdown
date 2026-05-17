import { writable } from 'svelte/store';

export type VideoMetadata = {
  duration: number;
  fps: number;
  codec: string;
  width: number;
  height: number;
  fileSize: number;
  audioCodec: string | null;
};

export type ExportStatus = {
  state: 'idle' | 'running' | 'success' | 'error';
  message: string;
  outputPath?: string;
  outputSize?: number;
};

export type EditorState = {
  currentFile: string | null;
  videoSrc: string | null;
  metadata: VideoMetadata | null;
  currentTime: number;
  inPoint: number;
  outPoint: number;
  exportStatus: ExportStatus;
};

const initialState: EditorState = {
  currentFile: null,
  videoSrc: null,
  metadata: null,
  currentTime: 0,
  inPoint: 0,
  outPoint: 0,
  exportStatus: {
    state: 'idle',
    message: 'Choose a clip to begin.',
  },
};

export const editor = writable<EditorState>(initialState);

export function resetEditor(): void {
  editor.set(initialState);
}
