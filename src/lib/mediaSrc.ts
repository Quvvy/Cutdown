import { convertFileSrc } from '@tauri-apps/api/core';

/**
 * WebView2 treats `#` and `?` in a media URL as the start of a fragment/query.
 * `convertFileSrc` uses encodeURI, which does not escape those characters.
 */
export function mediaSrcFromPath(filePath: string): string {
  const converted = convertFileSrc(filePath);
  return converted.replace(/#/g, '%23').replace(/\?/g, '%3F');
}
