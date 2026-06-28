import { describe, expect, it } from 'vitest';
import {
  AUDIO_PRESETS,
  audioPresetExtension,
  audioPresetLabel,
  DEFAULT_AUDIO_PRESET_ID,
  migrateLegacyAudioFormat,
  resolveAudioPresetId,
} from './audioPresets';

describe('audioPresets', () => {
  it('exposes four built-in presets', () => {
    expect(AUDIO_PRESETS.map((preset) => preset.id)).toEqual([
      'audio-wav',
      'audio-mp3-192',
      'audio-mp3-128',
      'audio-ogg-192',
    ]);
  });

  it('maps preset ids to extensions and labels', () => {
    expect(audioPresetExtension('audio-wav')).toBe('wav');
    expect(audioPresetExtension('audio-mp3-128')).toBe('mp3');
    expect(audioPresetLabel('audio-mp3-192')).toBe('MP3 (192 kbps)');
  });

  it('migrates legacy audio format values', () => {
    expect(migrateLegacyAudioFormat('wav')).toBe('audio-wav');
    expect(migrateLegacyAudioFormat('mp3')).toBe(DEFAULT_AUDIO_PRESET_ID);
    expect(migrateLegacyAudioFormat('ogg')).toBe('audio-ogg-192');
  });

  it('falls back to the default preset for unknown ids', () => {
    expect(resolveAudioPresetId('unknown')).toBe(DEFAULT_AUDIO_PRESET_ID);
  });
});
