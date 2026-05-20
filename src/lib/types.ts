export type ClipHistoryEntry = {
  outputPath: string;
  sourcePath: string | null;
  presetId: string;
  exportedAt: string;
  fileSize: number;
  duration: number;
};

export type NormalizedCropRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};
