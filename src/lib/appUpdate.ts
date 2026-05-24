import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdateCheckResult =
  | { status: 'available'; update: Update }
  | { status: 'upToDate' }
  | { status: 'error'; message: string };

export type UpdateInstallProgress = {
  downloadedBytes: number;
  totalBytes: number | null;
};

export async function checkForAppUpdate(): Promise<UpdateCheckResult> {
  try {
    const update = await check();
    if (update) {
      return { status: 'available', update };
    }
    return { status: 'upToDate' };
  } catch (error) {
    return {
      status: 'error',
      message: error instanceof Error ? error.message : String(error),
    };
  }
}

export async function installAppUpdate(
  update: Update,
  onProgress: (progress: UpdateInstallProgress) => void,
): Promise<void> {
  let downloadedBytes = 0;
  let totalBytes: number | null = null;

  await update.downloadAndInstall((event) => {
    if (event.event === 'Started') {
      totalBytes = event.data.contentLength ?? null;
      onProgress({ downloadedBytes: 0, totalBytes });
      return;
    }

    if (event.event === 'Progress') {
      downloadedBytes += event.data.chunkLength;
      onProgress({ downloadedBytes, totalBytes });
      return;
    }

    if (event.event === 'Finished') {
      onProgress({ downloadedBytes, totalBytes });
    }
  });

  await relaunch();
}

export function formatUpdateBytes(bytes: number): string {
  if (bytes < 1024) {
    return `${bytes} B`;
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KB`;
  }
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
