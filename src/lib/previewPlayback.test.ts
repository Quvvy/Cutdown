import { describe, expect, it } from 'vitest';
import { nativePreviewSupported, planPreview } from './previewPlayback';

describe('planPreview', () => {
  it('uses native playback for 8-bit H.264 MP4/MOV', () => {
    expect(planPreview({ codec: 'h264', pixelFormat: 'yuv420p', container: 'mp4' })).toBe('native');
    expect(planPreview({ codec: 'avc1', pixelFormat: '', container: 'MOV' })).toBe('native');
  });

  it('remuxes H.264 inside MKV/TS so WebView2 can play it', () => {
    expect(planPreview({ codec: 'h264', pixelFormat: 'yuv420p', container: 'mkv' })).toBe('remux');
    expect(planPreview({ codec: 'h264', pixelFormat: 'yuv420p', container: 'ts' })).toBe('remux');
  });

  it('proxies HEVC/AV1/10-bit instead of showing a black frame', () => {
    expect(planPreview({ codec: 'hevc', pixelFormat: 'yuv420p', container: 'mp4' })).toBe('proxy');
    expect(planPreview({ codec: 'h264', pixelFormat: 'yuv420p10le', container: 'mp4' })).toBe('proxy');
    expect(planPreview({ codec: 'av1', pixelFormat: 'yuv420p', container: 'mp4' })).toBe('proxy');
    expect(nativePreviewSupported({ codec: 'hevc', container: 'mp4' })).toBe(false);
  });

  it('plays VP9 WebM natively and proxies VP9 in MKV', () => {
    expect(planPreview({ codec: 'vp9', pixelFormat: 'yuv420p', container: 'webm' })).toBe('native');
    expect(planPreview({ codec: 'vp9', pixelFormat: 'yuv420p', container: 'mkv' })).toBe('proxy');
  });
});
