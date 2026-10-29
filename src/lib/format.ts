export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) {
    return '00:00:00.0';
  }

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const wholeSeconds = Math.floor(seconds % 60);
  const tenths = Math.floor((seconds % 1) * 10);

  return `${hours.toString().padStart(2, '0')}:${minutes
    .toString()
    .padStart(2, '0')}:${wholeSeconds.toString().padStart(2, '0')}.${tenths}`;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '0 B';
  }

  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const exponent = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** exponent;

  return `${value.toFixed(value >= 10 || exponent === 0 ? 0 : 1)} ${units[exponent]}`;
}

export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

/** Drive-letter (`C:\`) or UNC (`\\server`) paths, or anything already using backslashes. */
function isWindowsStylePath(path: string): boolean {
  return /^[a-z]:/i.test(path) || path.includes('\\');
}

export function splitOutputPath(path: string): { directory: string; fileName: string } {
  if (!path) {
    return { directory: '', fileName: 'cutdown.mp4' };
  }

  if (!isWindowsStylePath(path)) {
    const fileName = path.split('/').pop() || 'cutdown.mp4';
    const directory = path.slice(0, path.length - fileName.length);
    return { directory: directory === '/' ? '/' : directory.replace(/\/+$/, ''), fileName };
  }

  const normalized = path.replace(/\//g, '\\');
  const fileName = normalized.split('\\').pop() ?? 'cutdown.mp4';
  const directory = normalized.slice(0, normalized.length - fileName.length).replace(/\\$/, '');

  return { directory, fileName };
}

export function joinOutputPath(directory: string, fileName: string, defaultExtension = 'mp4'): string {
  const safeName = sanitizeExportFileName(fileName, defaultExtension);

  if (!directory) {
    return safeName;
  }

  if (!isWindowsStylePath(directory)) {
    return `${directory.replace(/\/+$/, '')}/${safeName}`;
  }

  const trimmedDir = directory.replace(/\\+$/, '');
  return `${trimmedDir}\\${safeName}`;
}

export function sanitizeExportFileName(fileName: string, defaultExtension = 'mp4'): string {
  const extension = defaultExtension.replace(/^\./, '').toLowerCase();
  let cleaned = fileName.replace(/[<>:"|?*\\/]/g, '_').trim();

  if (!cleaned) {
    cleaned = `cutdown.${extension}`;
  }

  if (!/\.[a-z0-9]{2,5}$/i.test(cleaned)) {
    cleaned = `${cleaned}.${extension}`;
  }

  return cleaned;
}
