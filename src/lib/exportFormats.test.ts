import { describe, expect, it } from 'vitest';
import { ensureExportFileExtension, extensionForExport, replaceFileExtension } from './exportFormats';

describe('exportFormats', () => {
  it('maps export type to the expected extension', () => {
    expect(extensionForExport('video', 'audio-mp3-192')).toBe('mp4');
    expect(extensionForExport('audio', 'audio-wav')).toBe('wav');
    expect(extensionForExport('audio', 'audio-ogg-192')).toBe('ogg');
  });

  it('replaces file extensions while preserving the stem', () => {
    expect(replaceFileExtension('clip-cutdown.mp4', 'mp3')).toBe('clip-cutdown.mp3');
    expect(replaceFileExtension('clip-cutdown', 'wav')).toBe('clip-cutdown.wav');
  });

  it('forces the target extension even when one already exists', () => {
    expect(ensureExportFileExtension('clip-cutdown.mp4', 'mp3')).toBe('clip-cutdown.mp3');
    expect(ensureExportFileExtension('clip-cutdown', 'ogg')).toBe('clip-cutdown.ogg');
  });
});
