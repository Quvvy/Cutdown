import type { AudioFormat } from './exportFormats';

export const DEFAULT_AUDIO_PRESET_ID = 'audio-mp3-192';

export type AudioPresetInfo = {
  id: string;
  name: string;
  description: string;
  format: AudioFormat;
  extension: string;
};

export const AUDIO_PRESETS: AudioPresetInfo[] = [
  {
    id: 'audio-wav',
    name: 'WAV (lossless)',
    description: 'Best quality · larger files · editing',
    format: 'wav',
    extension: 'wav',
  },
  {
    id: 'audio-mp3-192',
    name: 'MP3 (192 kbps)',
    description: 'Recommended · plays everywhere',
    format: 'mp3',
    extension: 'mp3',
  },
  {
    id: 'audio-mp3-128',
    name: 'MP3 (128 kbps)',
    description: 'Smaller files · voice / Discord',
    format: 'mp3',
    extension: 'mp3',
  },
  {
    id: 'audio-ogg-192',
    name: 'OGG (192 kbps)',
    description: 'Open format · good quality',
    format: 'ogg',
    extension: 'ogg',
  },
];

const AUDIO_PRESET_BY_ID = new Map(AUDIO_PRESETS.map((preset) => [preset.id, preset]));

export function getAudioPreset(id: string): AudioPresetInfo | null {
  return AUDIO_PRESET_BY_ID.get(id) ?? null;
}

export function resolveAudioPresetId(value: unknown): string {
  if (typeof value === 'string' && AUDIO_PRESET_BY_ID.has(value)) {
    return value;
  }
  return DEFAULT_AUDIO_PRESET_ID;
}

export function audioPresetExtension(id: string): string {
  return getAudioPreset(id)?.extension ?? 'mp3';
}

export function audioPresetLabel(id: string): string {
  return getAudioPreset(id)?.name ?? 'MP3 (192 kbps)';
}

export function audioPresetFormat(id: string): AudioFormat {
  return getAudioPreset(id)?.format ?? 'mp3';
}

export function migrateLegacyAudioFormat(format: unknown): string {
  if (format === 'wav') {
    return 'audio-wav';
  }
  if (format === 'ogg') {
    return 'audio-ogg-192';
  }
  if (format === 'mp3') {
    return DEFAULT_AUDIO_PRESET_ID;
  }
  return DEFAULT_AUDIO_PRESET_ID;
}
