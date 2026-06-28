import { joinOutputPath } from './format';

export type ExportSegmentRange = {
  start: number;
  end: number;
};

export type ExportJob = {
  id: string;
  outputPath: string;
  segments: ExportSegmentRange[];
  label: string;
};

export function buildPerSegmentJobs(
  baseDirectory: string,
  baseFileName: string,
  segments: ExportSegmentRange[],
  extension = 'mp4',
): ExportJob[] {
  const stem = baseFileName.replace(/\.[^.]+$/i, '') || 'cutdown';
  const safeExtension = extension.replace(/^\./, '').toLowerCase();

  return segments.map((segment, index) => {
    const suffix = String(index + 1).padStart(2, '0');
    const fileName = `${stem}-seg${suffix}.${safeExtension}`;
    const outputPath = joinOutputPath(baseDirectory, fileName);

    return {
      id: crypto.randomUUID(),
      outputPath,
      segments: [segment],
      label: `Segment ${index + 1}`,
    };
  });
}
