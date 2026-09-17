import { describe, expect, it } from 'vitest';
import { extensionOf, isOpenablePath, isProjectPath, isVideoPath } from './openPath';

describe('openPath', () => {
  it('recognizes video and project files from Windows paths', () => {
    expect(isVideoPath(String.raw`C:\Clips\match.MP4`)).toBe(true);
    expect(isProjectPath(String.raw`D:\edit.cutdown`)).toBe(true);
    expect(isOpenablePath(String.raw`C:\Clips\replay.mkv`)).toBe(true);
    expect(isOpenablePath('notes.txt')).toBe(false);
    expect(extensionOf(String.raw`C:\a.b\file.m4v`)).toBe('m4v');
  });
});
