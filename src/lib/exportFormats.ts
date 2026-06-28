import { sanitizeExportFileName } from './format';
import { audioPresetExtension } from './audioPresets';

export type ExportType = 'video' | 'audio';

export type AudioFormat = 'wav' | 'mp3' | 'ogg';

export const AUDIO_FORMATS: AudioFormat[] = ['wav', 'mp3', 'ogg'];

export type AudioFormatInfo = {
  id: AudioFormat;
  label: string;
  hint: string;
  extension: string;
};

export const AUDIO_FORMAT_INFO: Record<AudioFormat, AudioFormatInfo> = {
  wav: {
    id: 'wav',
    label: 'WAV',
    hint: 'Best quality · larger files · editing',
    extension: 'wav',
  },
  mp3: {
    id: 'mp3',
    label: 'MP3',
    hint: 'Small files · plays everywhere',
    extension: 'mp3',
  },
  ogg: {
    id: 'ogg',
    label: 'OGG',
    hint: 'Good quality · open format',
    extension: 'ogg',
  },
};

export function defaultVideoExtension(): string {
  return 'mp4';
}

export function extensionForExport(exportType: ExportType, audioPresetId: string): string {
  return exportType === 'audio' ? audioPresetExtension(audioPresetId) : defaultVideoExtension();
}

export function replaceFileExtension(fileName: string, extension: string): string {
  const normalized = fileName.trim();
  const stem = normalized.replace(/\.[^.]+$/i, '') || 'cutdown';
  return `${stem}.${extension.replace(/^\./, '')}`;
}

export function ensureExportFileExtension(fileName: string, extension: string): string {
  return sanitizeExportFileName(replaceFileExtension(fileName, extension), extension);
}

export function parseAudioFormat(value: unknown): AudioFormat | null {
  if (value === 'wav' || value === 'mp3' || value === 'ogg') {
    return value;
  }
  return null;
}

export function parseExportType(value: unknown): ExportType | null {
  if (value === 'video' || value === 'audio') {
    return value;
  }
  return null;
}
