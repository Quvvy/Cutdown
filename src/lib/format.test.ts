import { describe, expect, it } from 'vitest';
import { joinOutputPath, splitOutputPath } from './format';

describe('splitOutputPath', () => {
  it('splits Windows paths and normalises forward slashes', () => {
    expect(splitOutputPath('E:\\clips\\a.mp4')).toEqual({ directory: 'E:\\clips', fileName: 'a.mp4' });
    expect(splitOutputPath('E:/clips/a.mp4')).toEqual({ directory: 'E:\\clips', fileName: 'a.mp4' });
  });

  it('keeps Linux paths as they are', () => {
    expect(splitOutputPath('/home/me/Videos/a.mp4')).toEqual({ directory: '/home/me/Videos', fileName: 'a.mp4' });
    expect(splitOutputPath('/a.mp4')).toEqual({ directory: '/', fileName: 'a.mp4' });
  });
});

describe('joinOutputPath', () => {
  it('joins with a backslash for Windows folders', () => {
    expect(joinOutputPath('E:\\clips\\', 'a.mp4')).toBe('E:\\clips\\a.mp4');
  });

  it('joins with a slash for Linux folders', () => {
    expect(joinOutputPath('/home/me/Videos/', 'a.mp4')).toBe('/home/me/Videos/a.mp4');
    expect(joinOutputPath('/', 'a.mp4')).toBe('/a.mp4');
  });

  it('round-trips through splitOutputPath', () => {
    const { directory, fileName } = splitOutputPath('/run/media/me/Clips/replay 1.mp4');
    expect(joinOutputPath(directory, fileName)).toBe('/run/media/me/Clips/replay 1.mp4');
  });
});
