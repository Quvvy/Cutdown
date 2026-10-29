import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { isWindows } from './platform';

/** Loopback media server base URL (Linux only; see src-tauri/src/media_server.rs). */
let mediaServerBase: string | null = null;

/**
 * WebKitGTK can't play <video> from asset://, so on Linux the backend serves clips over
 * http://127.0.0.1. Call once before the first mediaSrcFromPath.
 */
export async function initMediaSource(): Promise<void> {
  if (isWindows) {
    return;
  }
  try {
    mediaServerBase = await invoke<string | null>('media_base_url');
  } catch {
    mediaServerBase = null;
  }
}

/**
 * WebView2 treats `#` and `?` in a media URL as the start of a fragment/query.
 * `convertFileSrc` uses encodeURI, which does not escape those characters.
 */
export function mediaSrcFromPath(filePath: string): string {
  if (mediaServerBase) {
    return `${mediaServerBase}${encodeURIComponent(filePath)}`;
  }
  const converted = convertFileSrc(filePath);
  return converted.replace(/#/g, '%23').replace(/\?/g, '%3F');
}
