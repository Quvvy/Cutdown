export const VIDEO_EXTENSIONS = ['mp4', 'mkv', 'mov', 'webm', 'ts', 'avi', 'flv', 'm4v'] as const;
export const PROJECT_EXTENSION = 'cutdown';

export function extensionOf(path: string): string {
  const leaf = path.split(/[\\/]/).pop() ?? '';
  const dot = leaf.lastIndexOf('.');
  if (dot <= 0 || dot === leaf.length - 1) {
    return '';
  }
  return leaf.slice(dot + 1).trim().toLowerCase();
}

export function isVideoPath(path: string): boolean {
  return (VIDEO_EXTENSIONS as readonly string[]).includes(extensionOf(path));
}

export function isProjectPath(path: string): boolean {
  return extensionOf(path) === PROJECT_EXTENSION;
}

export function isOpenablePath(path: string): boolean {
  return isVideoPath(path) || isProjectPath(path);
}
