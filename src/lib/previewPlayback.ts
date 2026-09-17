export type PreviewPlan = 'native' | 'remux' | 'proxy';

export type PreviewSourceInfo = {
  codec?: string | null;
  pixelFormat?: string | null;
  container?: string | null;
  fileSize?: number | null;
};

export function planPreview(info: PreviewSourceInfo): PreviewPlan {
  const codec = (info.codec ?? '').trim().toLowerCase();
  const pixelFormat = (info.pixelFormat ?? '').trim().toLowerCase();
  const container = (info.container ?? '').trim().replace(/^\./, '').toLowerCase();

  if (!pixelFormatWebViewSafe(pixelFormat)) {
    return 'proxy';
  }

  if (isH264(codec)) {
    return containerIsMp4Family(container) ? 'native' : 'remux';
  }

  if (isVp8OrVp9(codec) && container === 'webm') {
    return 'native';
  }

  return 'proxy';
}

export function nativePreviewSupported(info: PreviewSourceInfo): boolean {
  return planPreview(info) === 'native';
}

function isH264(codec: string): boolean {
  return codec === 'h264' || codec === 'avc1' || codec === 'avc' || codec.startsWith('avc1.');
}

function isVp8OrVp9(codec: string): boolean {
  return codec === 'vp8' || codec === 'vp9' || codec === 'vp09';
}

function pixelFormatWebViewSafe(pixelFormat: string): boolean {
  return (
    pixelFormat === '' ||
    pixelFormat === 'yuv420p' ||
    pixelFormat === 'yuvj420p' ||
    pixelFormat === 'nv12'
  );
}

function containerIsMp4Family(container: string): boolean {
  return container === 'mp4' || container === 'm4v' || container === 'mov' || container === 'm4a' || container === '3gp';
}
