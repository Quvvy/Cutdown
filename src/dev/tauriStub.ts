// Lets the editor run in a plain browser during `npm run dev`, so UI changes can be
// checked without the desktop app. Imported only when Tauri is absent in dev mode.
// Clip metadata, waveform and file access come from dev/viteDevBackend.ts (real
// ffprobe/ffmpeg); export, upload and other OS features report that they need the app.

type Args = Record<string, unknown> | undefined;

const PATH_KEY = 'cutdown-dev-last-path';

function devUrl(endpoint: string, params: Record<string, string>): string {
  return `/__dev/${endpoint}?${new URLSearchParams(params)}`;
}

async function fetchJson<T>(url: string): Promise<T> {
  const response = await fetch(url);
  const body = await response.json();
  if (!response.ok) {
    throw new Error(body.error ?? `${url} failed`);
  }
  return body as T;
}

function askForPath(): string | null {
  let last = '';
  try {
    last = localStorage.getItem(PATH_KEY) ?? '';
  } catch {
    // storage blocked; no remembered path
  }
  const path = window.prompt('Dev mode: full path to a video file', last)?.trim().replace(/^"|"$/g, '');
  if (!path) {
    return null;
  }
  try {
    localStorage.setItem(PATH_KEY, path);
  } catch {
    // storage blocked
  }
  return path;
}

const defaultSettings = {
  watchFolder: null,
  watchFolderEnabled: false,
  lastExportDir: null,
  defaultExportDir: null,
  lastPresetId: 'lossless',
  preferGpuEncoding: false,
  runAtStartup: false,
  startMinimizedToTray: false,
  closeToTray: false,
  catboxUserHash: null,
  catboxApiUrl: null,
  recentSources: [] as string[],
  uploadProviders: [],
  defaultUploadProviderId: null,
  customExportPresets: [],
};

const needsApp = (feature: string) => () => {
  throw new Error(`${feature} needs the desktop app (npm run tauri dev).`);
};

const handlers: Record<string, (args: Args) => unknown> = {
  'plugin:dialog|open': () => askForPath(),
  'plugin:app|version': () => '0.0.0-dev',
  get_settings: () => defaultSettings,
  get_launch_path: () => null,
  check_ffmpeg: () => ({ available: true, ffmpegPath: 'dev', ffprobePath: 'dev', source: 'dev', message: 'Dev backend' }),
  // Mirrors builtin_presets() in src-tauri/src/presets.rs.
  list_presets: () => [
    { id: 'lossless-trim', name: 'Lossless Trim', description: 'Fast stream-copy trim with no re-encode.', lossless: true, requiresGpu: false, custom: false, targetBytes: null },
    { id: 'discord', name: 'Discord', description: "H.264/AAC sized to stay under Discord's 20 MB limit.", lossless: false, requiresGpu: false, custom: false, targetBytes: 18 * 1024 * 1024 },
    { id: 'archive', name: 'Archive', description: 'High-quality H.264 re-encode for keeping clips.', lossless: false, requiresGpu: false, custom: false, targetBytes: null },
    { id: 'twitter', name: 'Twitter / X', description: '720p H.264 with platform-friendly limits.', lossless: false, requiresGpu: false, custom: false, targetBytes: null },
  ],
  detect_gpu_encoders: () => [],
  list_clip_history: () => [],
  list_upload_providers: () => [],
  push_recent_source: (args) => {
    defaultSettings.recentSources = [String(args?.path), ...defaultSettings.recentSources.filter((p) => p !== args?.path)].slice(0, 10);
    return defaultSettings.recentSources;
  },
  path_exists: (args) => fetchJson<boolean>(devUrl('exists', { path: String(args?.path ?? '') })),
  probe_video: (args) => fetchJson(devUrl('probe', { path: String(args?.path ?? '') })),
  extract_waveform: (args) =>
    fetchJson(devUrl('waveform', { path: String(args?.path ?? ''), buckets: String(args?.bucketCount ?? 2400) })),
  extract_thumbnail: (args) =>
    fetchJson(
      devUrl('thumbnail', { path: String(args?.path ?? ''), time: String(args?.time ?? 0), height: String(args?.height ?? 72) }),
    ),
  // The browser plays the source directly; no remux/proxy.
  prepare_preview: (args) => {
    const params = args?.params as { inputPath: string } | undefined;
    return { previewPath: params?.inputPath ?? '', strategy: 'Preview remux' };
  },
  export_clip: needsApp('Export'),
  upload_file: needsApp('Upload'),
  install_ffmpeg: needsApp('Installing ffmpeg'),
  load_project_file: needsApp('Opening projects'),
  save_project_file: needsApp('Saving projects'),
};

let nextCallbackId = 1;
const callbacks = new Map<number, (payload: unknown) => void>();

(window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  metadata: {
    currentWindow: { label: 'main' },
    currentWebview: { windowLabel: 'main', label: 'main' },
  },
  plugins: {},
  // Path in the URL path, not a query: mediaSrcFromPath escapes `?` for WebView2.
  convertFileSrc: (path: string) => `/__dev/file/${encodeURIComponent(path)}`,
  transformCallback: (callback?: (payload: unknown) => void) => {
    const id = nextCallbackId++;
    if (callback) {
      callbacks.set(id, callback);
    }
    return id;
  },
  unregisterCallback: (id: number) => callbacks.delete(id),
  runCallback: (id: number, payload: unknown) => callbacks.get(id)?.(payload),
  callbacks,
  invoke: async (cmd: string, args?: Args) => {
    const handler = handlers[cmd];
    if (handler) {
      return handler(args);
    }
    // Event listeners, window title, notifications, clipboard, etc.: accept and ignore.
    if (cmd === 'plugin:event|listen') {
      return nextCallbackId++;
    }
    return null;
  },
};

console.info('[cutdown] Running with the dev Tauri stub. Open a clip with the folder button or Ctrl+O.');

export {};
