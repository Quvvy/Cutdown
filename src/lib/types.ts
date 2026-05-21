export type ClipHistoryEntry = {
  outputPath: string;
  sourcePath: string | null;
  presetId: string;
  exportedAt: string;
  fileSize: number;
  duration: number;
  shareUrl?: string | null;
};

export type NormalizedCropRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type TimelineBookmark = {
  id: string;
  time: number;
  label?: string;
};
