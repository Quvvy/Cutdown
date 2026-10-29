<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { getVersion } from '@tauri-apps/api/app';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { confirm, open, save } from '@tauri-apps/plugin-dialog';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
  import { onMount, tick } from 'svelte';
  import { get } from 'svelte/store';
  import ClipHistoryDrawer from './components/ClipHistoryDrawer.svelte';
  import ConfirmModal from './components/ConfirmModal.svelte';
  import EditorToolbar from './components/EditorToolbar.svelte';
  import ExportModal from './components/ExportModal.svelte';
  import ExportPanel from './components/ExportPanel.svelte';
  import RelinkSourceModal from './components/RelinkSourceModal.svelte';
  import SettingsModal from './components/SettingsModal.svelte';
  import MarkerLabelModal from './components/MarkerLabelModal.svelte';
  import PreviewToolbar from './components/PreviewToolbar.svelte';
  import ShortcutsModal from './components/ShortcutsModal.svelte';
  import StatusBar from './components/StatusBar.svelte';
  import Timeline from './components/Timeline.svelte';
  import ToastHost from './components/ToastHost.svelte';
  import TransportRow from './components/TransportRow.svelte';
  import UploadTargetModal from './components/UploadTargetModal.svelte';
  import UpdateModal from './components/UpdateModal.svelte';
  import VideoPreview from './components/VideoPreview.svelte';
  import { checkForAppUpdate, installAppUpdate } from './lib/appUpdate';
  import { isWindows } from './lib/platform';
  import {
    clamp,
    formatBytes,
    formatTime,
    joinOutputPath,
    sanitizeExportFileName,
    splitOutputPath,
  } from './lib/format';
  import type { ClipHistoryEntry, NormalizedCropRect, TimelineBookmark } from './lib/types';
  import {
    parseCustomPresetsFromSettings,
    type CustomExportPreset,
  } from './lib/exportPresets';
  import { resolveShortcut, type ShortcutAction } from './lib/editorShortcuts';
  import { coarseToFineOrder, thumbnailCount, thumbnailTimes } from './lib/thumbnailPlan';
  import { quickExportPath, quickExportRanges, remainingLabel, totalSeconds } from './lib/quickExport';
  import { buildPerSegmentJobs, type ExportJob } from './lib/exportQueue';
  import {
    audioPresetFormat,
    audioPresetLabel,
    DEFAULT_AUDIO_PRESET_ID,
    migrateLegacyAudioFormat,
    resolveAudioPresetId,
  } from './lib/audioPresets';
  import {
    ensureExportFileExtension,
    extensionForExport,
    parseExportType,
    type ExportType,
  } from './lib/exportFormats';
  import { initMediaSource, mediaSrcFromPath } from './lib/mediaSrc';
  import { isOpenablePath, isProjectPath, VIDEO_EXTENSIONS } from './lib/openPath';
  import { planPreview } from './lib/previewPlayback';
  import {
    createCutdownProjectPayload,
    CUTDOWN_PROJECT_EXTENSION,
    type CutdownProject,
  } from './lib/projectFile';
  import { createSequencePlaybackDriver } from './lib/sequencePlayback';
  import {
    addTimelineBookmark,
    clearTimelineRange,
    createInitialTimelineSegments,
    deleteOutsideTimelineRange,
    deleteTimelineSegment,
    duplicateTimelineSegment,
    keepOnlyTimelineRange,
    removeTimelineBookmark,
    reorderTimelineSegment,
    setTimelineRangeMarker,
    splitTimelineSegments,
    updateTimelineBookmarkLabel,
  } from './lib/timelineEditing';
  import { sequenceToSourceTime, sourceToSequenceTime } from './lib/timelineMapping';
  import {
    createCatboxProvider,
    createFilegardenProvider,
    kindLabel,
    normalizeUploadSummaries,
    parseProvidersFromSettings,
    readDefaultUploadProviderId,
    readUploadProvidersFromAppSettings,
    type UploadProvider,
    type UploadProviderSummary,
  } from './lib/uploadProviders';
  import {
    editor,
    totalSegmentDuration,
    type EditorState,
    type TimelineSegment,
    type VideoMetadata,
  } from './stores/editor';
  import { pushToast } from './stores/toasts';
  import type { Update } from '@tauri-apps/plugin-updater';

  type ExportResult = {
    outputPath: string;
    fileSize: number;
    duration: number;
  };

  type ExportProgress = {
    stage: string;
    message: string;
    percent?: number | null;
  };

  type FfmpegCheckResult = {
    available: boolean;
    ffmpegPath: string;
    ffprobePath: string;
    source: string;
    message: string;
  };

  type PresetInfo = {
    id: string;
    name: string;
    description: string;
    lossless: boolean;
    requiresGpu: boolean;
    custom: boolean;
    targetBytes: number | null;
  };

  type AppSettings = {
    watchFolder: string | null;
    watchFolderEnabled: boolean;
    lastExportDir: string | null;
    defaultExportDir: string | null;
    lastPresetId: string;
    preferGpuEncoding: boolean;
    runAtStartup: boolean;
    startMinimizedToTray: boolean;
    closeToTray?: boolean;
    catboxUserHash: string | null;
    catboxApiUrl: string | null;
    recentSources: string[];
    uploadProviders: UploadProvider[];
    defaultUploadProviderId: string | null;
    customExportPresets: CustomExportPreset[];
  };

  type PreviewResult = {
    previewPath: string;
    strategy: 'Preview remux' | 'Preview proxy';
  };

  let preview:
    | {
        seekTo(seconds: number): void;
        togglePlayback(): void;
        playPlayback(): void;
        fitToView(): Promise<void>;
        remeasureViewport(): void;
        pausePlayback(options?: { emit?: boolean }): void;
        isReady(): boolean;
        zoomIn(): void;
        zoomOut(): void;
        resetView(): void;
      }
    | undefined;
  let timeline:
    | {
        zoomToSourceRange(start: number, end: number): void;
        zoomToFitView(): void;
      }
    | undefined;
  let exportModalOpen = false;
  let settingsModalOpen = false;
  let updateModalOpen = false;
  let pendingUpdate: Update | null = null;
  let updateInstallVersion = '';
  let updateInstallNotes = '';
  let updateInstalling = false;
  let updateInstallError = '';
  let updateDownloadedBytes = 0;
  let updateTotalBytes: number | null = null;
  let updateCheckRunning = false;
  let appVersion = '0.0.0';
  let settingsInitialTab: 'general' | 'folders' | 'presets' | 'upload' = 'general';
  let shortcutsModalOpen = false;
  let historyDrawerOpen = false;
  let recentSources: string[] = [];
  let dragOver = false;
  let accurateTrimExport = false;
  let stripAudioExport = false;
  let exportBatchPerSegment = false;
  let queueUploadAfterExport = false;
  let fadeInSeconds = 0;
  let fadeOutSeconds = 0;
  let previewPlaybackRate = 1;
  let exportQueueProcessing = false;
  let clipHistory: ClipHistoryEntry[] = [];
  let historyBusyPath: string | null = null;
  let cropEnabled = false;
  let previewPlaying = false;
  let cropAspect: 'free' | '16:9' | '9:16' | '4:3' | '1:1' | 'custom' = 'free';
  let cropLockAspect = false;
  let cropCustomAspectW = 16;
  let cropCustomAspectH = 9;
  let cropRect: NormalizedCropRect = { x: 0.05, y: 0.05, width: 0.9, height: 0.9 };
  let uploadProviders: UploadProvider[] = [];
  let uploadProviderSummaries: UploadProviderSummary[] = [];
  let uploadPickerProviders: UploadProviderSummary[] = [];
  let defaultUploadProviderId: string | null = null;
  let selectedUploadProviderId: string | null = null;
  let exportPresets: PresetInfo[] = [];
  let customExportPresets: CustomExportPreset[] = [];
  let exportPresetId = 'lossless-trim';
  let exportType: ExportType = 'video';
  let audioPresetId = DEFAULT_AUDIO_PRESET_ID;
  let gpuEncoders: string[] = [];
  let preferGpuEncoding = true;
  let defaultExportDir: string | null = null;
  let runAtStartup = false;
  let startMinimizedToTray = false;
  let closeToTray = true;
  let launchHandoffReady = false;
  let drainLaunchInFlight: Promise<void> | null = null;
  let exportMode: 'sequence' | 'range' = 'sequence';
  let exportPanelOpen = false;
  let exportStartedAt = 0;
  const QUICK_PRESET_IDS = ['lossless-trim', 'discord', 'twitter'];
  let rangeLoopPlayback = false;
  // Set once the user toggles loop themselves; stops automatic looping for this clip.
  let rangeLoopUserSet = false;
  // Start playing when a newly opened clip's preview is ready.
  let autoplayOnReady = false;
  // Short confirmation of the last edit, shown in the status bar instead of a toast
  // so nothing pops over the video while trimming.
  let editNote = '';
  let editNoteTimer: ReturnType<typeof setTimeout> | null = null;

  function noteEdit(message: string): void {
    editNote = message;
    if (editNoteTimer) {
      clearTimeout(editNoteTimer);
    }
    editNoteTimer = setTimeout(() => (editNote = ''), 4000);
  }
  const sequencePlayback = createSequencePlaybackDriver(
    {
      pauseVideo: (options) => preview?.pausePlayback(options),
      playVideo: () => preview?.playPlayback(),
      seekVideo: (time) => preview?.seekTo(time),
      updateCurrentTime: (time) => editor.update((state) => ({ ...state, currentTime: time })),
    },
    {
      getSegments: () => get(editor).segments,
      getSourceTime: () => get(editor).currentTime,
      isRangeLoop: () => rangeLoopPlayback,
      getPlaybackRate: () => previewPlaybackRate,
      afterAdvance: tick,
    },
  );
  let clipVolume = 1;
  let exportProgressPercent: number | null = null;
  let outputPath = '';
  let outputDirectory = '';
  let outputFileName = 'cutdown.mp4';
  let watchFolder: string | null = null;
  let watchFolderEnabled = false;
  let ffmpegStatus = '';
  let ffmpegAvailable = true;
  let ffmpegInstalling = false;
  let ffmpegInstallMessage = '';
  let ffmpegInstallPercent: number | null = null;
  let segmentHistory: Array<{
    segments: TimelineSegment[];
    selectedSegmentId: string | null;
  }> = [];
  let redoHistory: Array<{
    segments: TimelineSegment[];
    selectedSegmentId: string | null;
  }> = [];
  let previewFallbackRunning = false;
  // ~13 px/s until the clip opens and the timeline fits it (see timelineZoom.ts).
  let timelineZoom = 225;
  let videoTrackHeight = 68;
  let audioTrackHeight = 58;
  let workspaceEl: HTMLElement | undefined;
  let workspaceSplitter: HTMLButtonElement | undefined;
  let workspaceSplitRatio = 0.52;
  let workspaceResizing = false;
  let workspaceResizeStartY = 0;
  let workspaceResizeStartRatio = 0.52;

  const WORKSPACE_SPLIT_KEY = 'cutdown-workspace-split';
  const WORKSPACE_SPLITTER_PX = 6;
  const MIN_PREVIEW_PANE_PX = 120;
  const MIN_TIMELINE_PANE_PX = 160;
  let rangeStart: number | null = null;
  let rangeEnd: number | null = null;
  let bookmarks: TimelineBookmark[] = [];
  let selectedBookmarkId: string | null = null;
  let waveformPeaks: number[] = [];
  // Filmstrip frames, filled in coarse-to-fine; null until fetched.
  let thumbnails: (string | null)[] = [];
  let thumbnailsFor: string | null = null;
  let thumbnailGeneration = 0;
  let waveformLoading = false;
  let waveformLoadGeneration = 0;
  let pendingWaveform: { path: string; hasAudio: boolean; generation: number; duration: number } | null =
    null;
  let waveformDeferTimer: ReturnType<typeof setTimeout> | null = null;
  let markerLabelModalOpen = false;
  let editingBookmarkId: string | null = null;
  let currentWindowTitle = '';
  let openingClip = false;
  let openingMessage = 'Opening clip…';
  let lastShareUrl: string | null = null;
  let statusDismissed = false;
  let exportJobLabel = '';
  let exportJobIndex = 1;
  let exportJobTotal = 1;
  let uploadTargetModalOpen = false;
  let uploadTargetPath = '';
  let watchFolderConfirmOpen = false;
  let watchFolderPendingPath = '';
  let clearHistoryConfirmOpen = false;
  let relinkModalOpen = false;
  let relinkMissingPath = '';
  let relinkProjectName = '';
  let pendingProject: CutdownProject | null = null;
  let trayHintDismissed = false;

  const TRAY_HINT_KEY = 'cutdown-tray-hint-dismissed';
  const READY_HINT = 'Ready. Mark the part to keep with I and O (or cut with X), then export with Ctrl+E.';
  const TRAY_HINT_MESSAGE =
    'Closing the window keeps Cutdown running in the tray. Restore it from the tray icon, or change this in Settings → General.';
  const BOOKMARK_DEDUPE_SECONDS = 0.05;
  const BOOKMARK_REMOVE_TOLERANCE = 0.1;

  function loadWorkspaceSplit(): void {
    try {
      const saved = localStorage.getItem(WORKSPACE_SPLIT_KEY);
      if (!saved) {
        return;
      }

      const ratio = Number.parseFloat(saved);
      if (Number.isFinite(ratio) && ratio >= 0.2 && ratio <= 0.8) {
        workspaceSplitRatio = ratio;
      }
    } catch {
      // Ignore invalid persisted split.
    }
  }

  function saveWorkspaceSplit(): void {
    try {
      localStorage.setItem(WORKSPACE_SPLIT_KEY, String(workspaceSplitRatio));
    } catch {
      // Ignore storage failures.
    }
  }

  function startWorkspaceResize(event: PointerEvent): void {
    event.preventDefault();
    workspaceResizing = true;
    workspaceResizeStartY = event.clientY;
    workspaceResizeStartRatio = workspaceSplitRatio;
    (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  }

  function updateWorkspaceResize(event: PointerEvent): void {
    if (!workspaceResizing || !workspaceEl) {
      return;
    }

    const available = workspaceEl.clientHeight - WORKSPACE_SPLITTER_PX;
    if (available <= MIN_PREVIEW_PANE_PX + MIN_TIMELINE_PANE_PX) {
      return;
    }

    const delta = event.clientY - workspaceResizeStartY;
    const startPreviewPx = workspaceResizeStartRatio * available;
    const nextPreviewPx = clamp(
      startPreviewPx + delta,
      MIN_PREVIEW_PANE_PX,
      available - MIN_TIMELINE_PANE_PX,
    );
    workspaceSplitRatio = nextPreviewPx / available;
    preview?.remeasureViewport();
  }

  function stopWorkspaceResize(event: PointerEvent): void {
    if (!workspaceResizing) {
      return;
    }

    if (workspaceSplitter?.hasPointerCapture(event.pointerId)) {
      workspaceSplitter.releasePointerCapture(event.pointerId);
    }

    saveWorkspaceSplit();
    workspaceResizing = false;
    void tick().then(() => preview?.fitToView());
  }

  onMount(() => {
    loadWorkspaceSplit();
    try {
      trayHintDismissed = localStorage.getItem(TRAY_HINT_KEY) === '1';
    } catch {
      trayHintDismissed = false;
    }
    let unlistenSecondInstance: Promise<() => void> | null = null;
    void (async () => {
      unlistenSecondInstance = listen('second-instance', () => {
        if (launchHandoffReady) {
          void drainLaunchPaths();
        }
      });
      await unlistenSecondInstance;
      await bootstrapApp();
    })();

    // In-app updates ship Windows installers only; Linux updates come from the .deb/AppImage.
    const updateCheckTimer = isWindows
      ? window.setTimeout(() => {
          void checkForUpdates();
        }, 8000)
      : undefined;

    const unlistenFfmpegInstall = listen<{
      stage: string;
      message: string;
      percent?: number;
    }>('ffmpeg_install_progress', (event) => {
      ffmpegInstallMessage = event.payload.message;
      ffmpegInstallPercent =
        typeof event.payload.percent === 'number' ? event.payload.percent : ffmpegInstallPercent;
    });

    const unlistenExport = listen<ExportProgress>('export_progress', (event) => {
      exportProgressPercent =
        typeof event.payload.percent === 'number' ? event.payload.percent : exportProgressPercent;
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: event.payload.stage === 'complete' ? 'success' : 'running',
          message: event.payload.message,
          outputPath: state.exportStatus.outputPath,
          outputSize: state.exportStatus.outputSize,
        },
      }));
    });

    const unlistenWatch = listen<{ path: string }>('watch_folder_clip', (event) => {
      void handleWatchFolderClip(event.payload.path);
    });

    const appWindow = getCurrentWindow();
    const unlistenWindowResized = appWindow.onResized(() => {
      preview?.remeasureViewport();
    });

    const unlistenDragDrop = appWindow.onDragDropEvent((event) => {
      if (event.payload.type === 'over') {
        dragOver = true;
      } else if (event.payload.type === 'drop') {
        dragOver = false;
        const path = event.payload.paths.find((candidate) => isOpenablePath(candidate));
        if (path) {
          void openIncomingPath(path);
        }
      } else {
        dragOver = false;
      }
    });

    return () => {
      window.clearTimeout(updateCheckTimer);
      sequencePlayback.dispose();
      void unlistenFfmpegInstall.then((stop) => stop());
      void unlistenExport.then((stop) => stop());
      void unlistenWatch.then((stop) => stop());
      if (unlistenSecondInstance) {
        void unlistenSecondInstance.then((stop) => stop());
      }
      void unlistenWindowResized.then((stop) => stop());
      void unlistenDragDrop.then((stop) => stop());
      const previewTempPath = get(editor).previewTempPath;
      if (previewTempPath) {
        void invoke('cleanup_preview', { path: previewTempPath });
      }
    };
  });

  async function installFfmpeg(): Promise<void> {
    if (ffmpegInstalling) {
      return;
    }

    ffmpegInstalling = true;
    ffmpegInstallMessage = 'Starting ffmpeg download…';
    ffmpegInstallPercent = 0;

    try {
      const ffmpeg = await invoke<FfmpegCheckResult>('install_ffmpeg');
      ffmpegAvailable = ffmpeg.available;
      ffmpegStatus =
        gpuEncoders.length > 0
          ? `${ffmpeg.message} GPU: ${gpuEncoders.join(', ')}.`
          : ffmpeg.message;

      if (ffmpeg.available) {
        gpuEncoders = await invoke<string[]>('detect_gpu_encoders');
        if (gpuEncoders.length > 0) {
          ffmpegStatus = `${ffmpeg.message} GPU: ${gpuEncoders.join(', ')}.`;
        }
        pushToast('ffmpeg installed successfully.', 'success');
      } else {
        pushToast(ffmpeg.message, 'error');
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      ffmpegInstallMessage = message;
      pushToast(message, 'error');
    } finally {
      ffmpegInstalling = false;
      ffmpegInstallPercent = null;
    }
  }

  async function refreshAppVersion(): Promise<void> {
    try {
      appVersion = await getVersion();
    } catch {
      appVersion = '0.0.0';
    }
  }

  function openUpdateModal(update: Update): void {
    pendingUpdate = update;
    updateInstallVersion = update.version;
    updateInstallNotes = update.body ?? '';
    updateInstallError = '';
    updateInstalling = false;
    updateDownloadedBytes = 0;
    updateTotalBytes = null;
    updateModalOpen = true;
  }

  async function checkForUpdates(options: { manual?: boolean } = {}): Promise<void> {
    if (updateCheckRunning) {
      return;
    }

    updateCheckRunning = true;
    try {
      const result = await checkForAppUpdate();
      if (result.status === 'available') {
        openUpdateModal(result.update);
        return;
      }

      if (result.status === 'upToDate') {
        if (options.manual) {
          pushToast('Cutdown is up to date.', 'success');
        }
        return;
      }

      if (options.manual) {
        pushToast(result.message, 'error');
      }
    } finally {
      updateCheckRunning = false;
    }
  }

  async function installPendingUpdate(): Promise<void> {
    if (!pendingUpdate || updateInstalling) {
      return;
    }

    updateInstalling = true;
    updateInstallError = '';

    try {
      await installAppUpdate(pendingUpdate, ({ downloadedBytes, totalBytes }) => {
        updateDownloadedBytes = downloadedBytes;
        updateTotalBytes = totalBytes;
      });
    } catch (error) {
      updateInstalling = false;
      updateInstallError = error instanceof Error ? error.message : String(error);
      pushToast(updateInstallError, 'error');
    }
  }

  async function bootstrapApp(): Promise<void> {
    try {
      await Promise.all([refreshAppVersion(), initMediaSource()]);
      const [settings, ffmpeg, presets, encoders] = await Promise.all([
        invoke<AppSettings>('get_settings'),
        invoke<FfmpegCheckResult>('check_ffmpeg'),
        invoke<PresetInfo[]>('list_presets'),
        invoke<string[]>('detect_gpu_encoders'),
      ]);

      watchFolder = settings.watchFolder;
      watchFolderEnabled = settings.watchFolderEnabled;
      defaultExportDir = settings.defaultExportDir ?? settings.lastExportDir;
      exportPresetId = settings.lastPresetId || 'lossless-trim';
      preferGpuEncoding = settings.preferGpuEncoding;
      runAtStartup = settings.runAtStartup;
      startMinimizedToTray = settings.startMinimizedToTray;
      closeToTray = settings.closeToTray ?? true;
      showTrayHintOnce();
      uploadProviders = resolveUploadProvidersFromSettings(settings);
      const bootstrapSettingsRecord = settings as AppSettings & Record<string, unknown>;
      defaultUploadProviderId =
        readDefaultUploadProviderId(bootstrapSettingsRecord) ??
        uploadProviders.find((provider) => provider.enabled)?.id ??
        null;
      selectedUploadProviderId = defaultUploadProviderId;
      await refreshUploadProviderSummaries();
      exportPresets = presets;
      customExportPresets = parseCustomPresetsFromSettings(settings.customExportPresets);
      clipHistory = await invoke<ClipHistoryEntry[]>('list_clip_history');
      recentSources = settings.recentSources ?? [];
      gpuEncoders = encoders;
      ffmpegAvailable = ffmpeg.available;
      ffmpegStatus =
        encoders.length > 0
          ? `${ffmpeg.message} GPU: ${encoders.join(', ')}.`
          : ffmpeg.message;

      if (!ffmpeg.available) {
        editor.update((state) => ({
          ...state,
          exportStatus: {
            state: 'error',
            message: ffmpeg.message,
          },
        }));
      }

      await drainLaunchPaths();
      launchHandoffReady = true;
      await drainLaunchPaths();
    } catch (error) {
      ffmpegStatus = error instanceof Error ? error.message : String(error);
    }

    if (!(await isPermissionGranted())) {
      await requestPermission();
    }
  }

  // One-time tip instead of a permanent banner.
  function showTrayHintOnce(): void {
    if (!closeToTray || trayHintDismissed) {
      return;
    }
    pushToast(TRAY_HINT_MESSAGE, 'info', 12000);
    trayHintDismissed = true;
    try {
      localStorage.setItem(TRAY_HINT_KEY, '1');
    } catch {
      // Ignore storage failures.
    }
  }

  function restoreTrayHint(): void {
    trayHintDismissed = false;
    try {
      localStorage.removeItem(TRAY_HINT_KEY);
    } catch {
      // Ignore storage failures.
    }
    showTrayHintOnce();
  }

  $: cropLockedAspectRatio = (() => {
    if (!cropLockAspect || cropAspect === 'free') {
      return null;
    }
    if (cropAspect === '16:9') {
      return 16 / 9;
    }
    if (cropAspect === '9:16') {
      return 9 / 16;
    }
    if (cropAspect === '4:3') {
      return 4 / 3;
    }
    if (cropAspect === '1:1') {
      return 1;
    }
    const w = Math.max(1, cropCustomAspectW);
    const h = Math.max(1, cropCustomAspectH);
    return w / h;
  })();

  function dismissExportStatus(): void {
    statusDismissed = true;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'idle',
        message: state.currentFile ? 'Ready to edit.' : 'Choose a clip to begin.',
      },
    }));
  }

  async function cancelExport(): Promise<void> {
    await invoke('cancel_export').catch(() => undefined);
    editor.update((state) => ({
      ...state,
      exportStatus: {
        ...state.exportStatus,
        state: 'error',
        message: 'Export cancellation requested.',
      },
    }));
  }

  async function reloadUploadProvidersFromDisk(): Promise<void> {
    const [settings, editor, summaries] = await Promise.all([
      invoke<AppSettings>('get_settings'),
      invoke<{ providers: UploadProvider[]; defaultUploadProviderId: string | null }>(
        'get_upload_providers_for_editor',
      ),
      invoke<UploadProviderSummary[]>('list_upload_providers'),
    ]);

    const editorProviders = parseProvidersFromSettings(editor.providers);
    uploadProviders =
      editorProviders.length > 0 ? editorProviders : resolveUploadProvidersFromSettings(settings);
    defaultUploadProviderId =
      editor.defaultUploadProviderId ??
      readDefaultUploadProviderId(settings as AppSettings & Record<string, unknown>) ??
      uploadProviders.find((provider) => provider.enabled)?.id ??
      null;
    uploadProviderSummaries = normalizeUploadSummaries(summaries);
    syncUploadProviderSelection();
  }

  async function openUploadPicker(path: string): Promise<void> {
    uploadTargetPath = path;
    try {
      await reloadUploadProvidersFromDisk();
      uploadPickerProviders = getEnabledUploadTargets();
      selectedUploadProviderId =
        uploadPickerProviders.find((entry) => entry.id === defaultUploadProviderId)?.id ??
        uploadPickerProviders[0]?.id ??
        null;
    } catch (error) {
      uploadPickerProviders = [];
      pushToast(error instanceof Error ? error.message : String(error), 'error');
    }
    uploadTargetModalOpen = true;
  }

  async function copyShareLink(url: string): Promise<void> {
    await invoke('copy_text_to_clipboard', { text: url });
    pushToast('Share link copied to clipboard.', 'success');
  }

  async function notify(message: string, title = 'Cutdown'): Promise<void> {
    try {
      if (await isPermissionGranted()) {
        await sendNotification({ title, body: message });
      }
    } catch {
      // Notifications are optional.
    }
  }

  function hasUnsavedEdits(): boolean {
    return segmentHistory.length > 0;
  }

  async function handleWatchFolderClip(path: string): Promise<void> {
    await openIncomingPath(path);
  }

  function confirmWatchFolderReplace(): void {
    watchFolderConfirmOpen = false;
    const path = watchFolderPendingPath;
    watchFolderPendingPath = '';
    if (path) {
      void openIncomingPath(path, { confirmIfDirty: false });
    }
  }

  async function drainLaunchPaths(): Promise<void> {
    if (drainLaunchInFlight) {
      await drainLaunchInFlight;
    }

    drainLaunchInFlight = (async () => {
      while (true) {
        const path = await invoke<string | null>('get_launch_path');
        if (!path) {
          return;
        }
        await openIncomingPath(path);
      }
    })();

    try {
      await drainLaunchInFlight;
    } finally {
      drainLaunchInFlight = null;
    }
  }

  async function openIncomingPath(
    path: string,
    options: { confirmIfDirty?: boolean } = {},
  ): Promise<void> {
    if (options.confirmIfDirty !== false && hasUnsavedEdits() && $editor.currentFile) {
      watchFolderPendingPath = path;
      watchFolderConfirmOpen = true;
      return;
    }

    if (isProjectPath(path)) {
      await openProjectPath(path);
      return;
    }

    await openClipPath(path);
  }

  async function cleanupPreview(path: string | null): Promise<void> {
    if (!path) {
      return;
    }

    try {
      await invoke('cleanup_preview', { path });
    } catch {
      // Temporary preview cleanup is best-effort.
    }
  }

  $: metadata = $editor.metadata;
  $: duration = metadata?.duration ?? 0;
  $: segments = $editor.segments;
  $: selectedSegment = segments.find((segment) => segment.id === $editor.selectedSegmentId) ?? null;
  $: outputDuration = totalSegmentDuration(segments);
  $: fileName = $editor.currentFile?.split(/[\\/]/).pop() ?? 'No file selected';
  $: canExport = Boolean($editor.currentFile && outputDuration > 0);
  $: exportActivityVisible = exportQueueProcessing;
  $: uploadTargetsConfigured = getEnabledUploadTargets().length > 0;
  $: exportStatusLabel =
    exportActivityVisible && exportJobLabel
      ? [exportJobLabel, exportJobTotal > 1 ? `${exportJobIndex}/${exportJobTotal}` : '', $editor.exportStatus.message]
          .filter(Boolean)
          .join(' · ')
      : $editor.exportStatus.message;
  $: exportEta =
    $editor.exportStatus.state === 'running' ? remainingLabel((Date.now() - exportStartedAt) / 1000, exportProgressPercent) : null;
  $: rangeIsNarrowed = Boolean(
    normalizedRange && (normalizedRange.start > 0.05 || normalizedRange.end < duration - 0.05),
  );
  $: quickRanges = quickExportRanges(segments, rangeIsNarrowed ? normalizedRange : null);
  $: quickExportSeconds = totalSeconds(quickRanges);
  $: quickPresetIds = QUICK_PRESET_IDS.filter((id) => exportPresets.some((preset) => preset.id === id));
  $: quickDestination = $editor.currentFile
    ? quickExportPath($editor.currentFile, quickRanges[0]?.start ?? 0, extensionForExport('video', audioPresetId))
    : '';
  $: clipMetadataDetail = metadata
    ? [
        `${metadata.width}x${metadata.height}`,
        metadata.variableFrameRate
          ? `variable frame rate, ${metadata.fps.toFixed(1)} fps average, up to ${Math.round(metadata.maxFps ?? metadata.fps)}`
          : `${metadata.fps.toFixed(2)} fps`,
        metadata.codec,
        metadata.audioCodec ? `${metadata.audioCodec}${metadata.audioChannels ? ` ${metadata.audioChannels}ch` : ''}` : 'no audio',
        `preview: ${$editor.previewStrategy}`,
        formatBytes(metadata.fileSize),
      ].join(' · ')
    : '';
  $: statusDismissible =
    !statusDismissed &&
    ($editor.exportStatus.state === 'success' || $editor.exportStatus.state === 'error');
  $: normalizedRange =
    rangeStart !== null && rangeEnd !== null
      ? {
          start: Math.min(rangeStart, rangeEnd),
          end: Math.max(rangeStart, rangeEnd),
        }
      : null;
  $: rangeDuration = normalizedRange ? normalizedRange.end - normalizedRange.start : 0;
  $: canUseRange = Boolean(normalizedRange && rangeDuration > 0.05);
  $: selectedExportPreset = exportPresets.find((preset) => preset.id === exportPresetId) ?? null;
  $: streamCopyBlockers = selectedExportPreset?.lossless ? reencodeReasons : [];
  // Edits that make even a lossless preset re-encode.
  $: reencodeReasons = (() => {
    const blockers: string[] = [];
    if (exportCropPixels()) {
      blockers.push('crop');
    }
    if (Math.abs(clipVolume - 1) > 0.001) {
      blockers.push('volume');
    }
    if (accurateTrimExport) {
      blockers.push('accurate trim');
    }
    if (stripAudioExport) {
      blockers.push('strip audio');
    }
    if (fadeInSeconds > 0 || fadeOutSeconds > 0) {
      blockers.push('audio fade');
    }
    return blockers;
  })();
  $: usesStreamCopy = Boolean(selectedExportPreset?.lossless && streamCopyBlockers.length === 0);
  $: outputExtension = extensionForExport(exportType, audioPresetId);
  $: outputPath = joinOutputPath(outputDirectory, outputFileName, outputExtension);
  $: updateWindowTitle(fileName);

  function applyOutputPath(path: string): void {
    const parts = splitOutputPath(path);
    outputDirectory = parts.directory;
    outputFileName = parts.fileName;
  }

  function cloneSegments(source: TimelineSegment[]): TimelineSegment[] {
    return source.map((segment) => ({ ...segment }));
  }

  function updateWindowTitle(name: string): void {
    const nextTitle = `Cutdown - ${name}`;

    if (nextTitle === currentWindowTitle) {
      return;
    }

    currentWindowTitle = nextTitle;
    void getCurrentWindow().setTitle(nextTitle);
  }

  function pushUndoSnapshot(): void {
    segmentHistory = [
      ...segmentHistory.slice(-19),
      {
        segments: cloneSegments($editor.segments),
        selectedSegmentId: $editor.selectedSegmentId,
      },
    ];
    redoHistory = [];
  }

  function undoSegmentEdit(): void {
    const snapshot = segmentHistory[segmentHistory.length - 1];

    if (!snapshot) {
      return;
    }

    redoHistory = [
      ...redoHistory.slice(-19),
      {
        segments: cloneSegments($editor.segments),
        selectedSegmentId: $editor.selectedSegmentId,
      },
    ];
    segmentHistory = segmentHistory.slice(0, -1);
    editor.update((state) => ({
      ...state,
      segments: cloneSegments(snapshot.segments),
      selectedSegmentId: snapshot.selectedSegmentId,
    }));
    noteEdit('Undid last edit.');
  }

  function redoSegmentEdit(): void {
    const snapshot = redoHistory[redoHistory.length - 1];

    if (!snapshot) {
      return;
    }

    segmentHistory = [
      ...segmentHistory.slice(-19),
      {
        segments: cloneSegments($editor.segments),
        selectedSegmentId: $editor.selectedSegmentId,
      },
    ];
    redoHistory = redoHistory.slice(0, -1);
    editor.update((state) => ({
      ...state,
      segments: cloneSegments(snapshot.segments),
      selectedSegmentId: snapshot.selectedSegmentId,
    }));
    noteEdit('Redid edit.');
  }

  async function chooseClip(): Promise<void> {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Video and Cutdown projects',
          extensions: [...VIDEO_EXTENSIONS, 'cutdown'],
        },
        {
          name: 'Video',
          extensions: [...VIDEO_EXTENSIONS],
        },
        {
          name: 'Cutdown project',
          extensions: ['cutdown'],
        },
      ],
    });

    if (typeof selected !== 'string') {
      return;
    }

    await openIncomingPath(selected);
  }

  async function openClipPath(selected: string): Promise<void> {
    autoplayOnReady = true;
    if (!ffmpegAvailable) {
      const ffmpeg = await invoke<FfmpegCheckResult>('check_ffmpeg');
      ffmpegAvailable = ffmpeg.available;
      ffmpegStatus = ffmpeg.message;

      if (!ffmpeg.available) {
        editor.update((state) => ({
          ...state,
          exportStatus: {
            state: 'error',
            message: ffmpeg.message,
          },
        }));
        return;
      }
    }

    void cleanupPreview($editor.previewTempPath);

    segmentHistory = [];
    redoHistory = [];
    rangeStart = null;
    rangeEnd = null;
    bookmarks = [];
    selectedBookmarkId = null;
    waveformPeaks = [];
    thumbnailGeneration += 1;
    thumbnails = [];
    thumbnailsFor = null;
    cancelPendingWaveform();
    waveformLoadGeneration += 1;
    rangeLoopPlayback = false;
    rangeLoopUserSet = false;
    sequencePlayback.reset();
    clipVolume = 1;
    cropEnabled = false;
    cropRect = { x: 0.05, y: 0.05, width: 0.9, height: 0.9 };
    exportMode = 'sequence';
    openingClip = true;
    openingMessage = 'Reading clip…';
    exportProgressPercent = null;
    statusDismissed = false;
    editor.update((state) => ({
      ...state,
      currentFile: selected,
      videoSrc: null,
      previewTempPath: null,
      previewStrategy: 'Native preview',
      metadata: null,
      currentTime: 0,
      segments: [],
      selectedSegmentId: null,
      exportStatus: {
        state: 'running',
        message: 'Probing clip metadata...',
      },
    }));

    try {
      const probed = await invoke<VideoMetadata>('probe_video', { path: selected });
      rangeStart = 0;
      rangeEnd = probed.duration;
      const plan = planPreview({
        codec: probed.codec,
        pixelFormat: probed.pixelFormat,
        container: probed.container,
        fileSize: probed.fileSize,
      });
      editor.update((state) => ({
        ...state,
        metadata: probed,
        segments: createInitialTimelineSegments(probed.duration),
        selectedSegmentId: null,
        exportStatus: {
          state: 'idle',
          message: `Opened ${formatBytes(probed.fileSize)} ${probed.codec.toUpperCase()} clip.`,
        },
      }));
      // Whole clip across the timeline, so segments are big enough to grab.
      void tick().then(zoomToFit);

      if (plan === 'native') {
        editor.update((state) => ({
          ...state,
          videoSrc: mediaSrcFromPath(selected),
          previewStrategy: 'Native preview',
          exportStatus: {
            state: 'idle',
            message: READY_HINT,
          },
        }));
      } else {
        openingMessage =
          plan === 'proxy'
            ? `This ${probed.codec.toUpperCase()} file can't play here directly, so Cutdown is making a quick preview copy. Your export still uses the original.`
            : `Repackaging the ${probed.container?.toUpperCase() || 'video'} file so it can play here (no quality change)…`;
        editor.update((state) => ({
          ...state,
          exportStatus: {
            state: 'running',
            message:
              plan === 'proxy'
                ? `Making a preview copy of this ${probed.codec.toUpperCase()} file…`
                : 'Repackaging for preview…',
          },
        }));
        const result = await invoke<PreviewResult>('prepare_preview', {
          params: { inputPath: selected, forceProxy: plan === 'proxy' },
        });
        await cleanupPreview($editor.previewTempPath);
        editor.update((state) => ({
          ...state,
          videoSrc: mediaSrcFromPath(result.previewPath),
          previewTempPath: result.previewPath,
          previewStrategy: result.strategy as EditorState['previewStrategy'],
          exportStatus: {
            state: 'idle',
            message: READY_HINT,
          },
        }));
      }

      queueWaveformAfterPreview(selected, probed.audioCodec != null, probed.duration);
      syncExportDefaultsForClip();
      void invoke<string[]>('push_recent_source', { path: selected }).then((sources) => {
        recentSources = sources;
      });
    } catch (error) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: error instanceof Error ? error.message : String(error),
        },
      }));
      pushToast(error instanceof Error ? error.message : String(error), 'error');
    } finally {
      openingClip = false;
    }
  }

  function handlePreviewEnded(): void {
    sequencePlayback.onEnded();
  }

  function handlePreviewPlayState(playing: boolean): void {
    previewPlaying = playing;
    sequencePlayback.onPlayState(playing);
  }

  function handlePreviewTimeUpdate(sourceTime: number): void {
    sequencePlayback.onTimeUpdate(sourceTime);
  }

  function seekTo(seconds: number): void {
    const nextTime = clamp(seconds, 0, duration);
    preview?.seekTo(nextTime);
    editor.update((state) => ({ ...state, currentTime: nextTime }));
    sequencePlayback.onUserSeek(nextTime);
  }

  function seekBySequence(deltaSeconds: number): void {
    if ($editor.segments.length === 0) {
      seekTo($editor.currentTime + deltaSeconds);
      return;
    }

    const sequenceTime = sourceToSequenceTime($editor.segments, $editor.currentTime);
    seekTo(sequenceToSourceTime($editor.segments, clamp(sequenceTime + deltaSeconds, 0, outputDuration)));
  }

  function refreshPreviewPlaybackAfterSegmentEdit(): void {
    sequencePlayback.onSegmentsChanged();
  }

  function selectSegment(id: string | null): void {
    selectedBookmarkId = null;
    editor.update((state) => ({ ...state, selectedSegmentId: id }));
  }

  function deselectSegment(): void {
    selectSegment(null);
  }

  function splitAtTime(splitTime: number, options: { recordUndo?: boolean; toast?: boolean } = {}): void {
    const { recordUndo = true, toast = true } = options;
    if (!canExport) {
      return;
    }

    const result = splitTimelineSegments($editor.segments, splitTime);
    if (!result.changed) {
      noteEdit('Move the playhead inside a segment to cut.');
      return;
    }

    if (recordUndo) {
      pushUndoSnapshot();
    }
    editor.update((state) => ({
      ...state,
      segments: result.segments,
      selectedSegmentId: result.selectedSegmentId,
    }));
    if (toast) {
      noteEdit(`Cut at ${formatTime(splitTime)}.`);
    }
    refreshPreviewPlaybackAfterSegmentEdit();
  }

  function splitAtCurrentTime(): void {
    splitAtTime($editor.currentTime);
  }

  function deleteSelectedSegment(): void {
    if (!$editor.selectedSegmentId) {
      return;
    }

    const result = deleteTimelineSegment($editor.segments, $editor.selectedSegmentId);
    if (result.status === 'minimum') {
      noteEdit('At least one segment is required.');
      return;
    }

    if (!result.changed) {
      return;
    }

    pushUndoSnapshot();
    editor.update((state) => ({
      ...state,
      segments: result.segments,
      selectedSegmentId: result.selectedSegmentId,
    }));
    noteEdit('Deleted selected segment.');
    refreshPreviewPlaybackAfterSegmentEdit();
  }

  function duplicateSelectedSegment(): void {
    if (!$editor.selectedSegmentId) {
      return;
    }

    const result = duplicateTimelineSegment($editor.segments, $editor.selectedSegmentId);

    if (!result.changed) {
      return;
    }

    pushUndoSnapshot();
    editor.update((state) => ({
      ...state,
      segments: result.segments,
      selectedSegmentId: result.selectedSegmentId,
    }));
    noteEdit('Duplicated selected segment.');
    refreshPreviewPlaybackAfterSegmentEdit();
  }

  function reorderSegment(id: string, toIndex: number): void {
    pushUndoSnapshot();
    const result = reorderTimelineSegment($editor.segments, id, toIndex);
    editor.update((state) => {
      if (!result.changed) {
        return state;
      }

      return {
        ...state,
        segments: result.segments,
        selectedSegmentId: result.selectedSegmentId,
      };
    });
    noteEdit('Moved segment.');
    refreshPreviewPlaybackAfterSegmentEdit();
  }

  function resizeSegment(
    id: string,
    sourceStart: number,
    sourceEnd: number,
    options: { recordUndo?: boolean; commit?: boolean } = {},
  ): void {
    const { recordUndo = false, commit = false } = options;
    if (!canExport) {
      return;
    }

    const targetSegment = $editor.segments.find((segment) => segment.id === id);
    if (!targetSegment) {
      return;
    }

    if (
      sourceStart === targetSegment.sourceStart &&
      sourceEnd === targetSegment.sourceEnd &&
      !commit
    ) {
      if (recordUndo) {
        pushUndoSnapshot();
      }
      return;
    }

    if (recordUndo) {
      pushUndoSnapshot();
    }

    editor.update((state) => {
      const nextSegments = state.segments.map((segment) =>
        segment.id === id ? { ...segment, sourceStart, sourceEnd } : segment,
      );
      const resizedSegment = nextSegments.find((segment) => segment.id === id);
      let nextCurrentTime = state.currentTime;

      if (
        resizedSegment &&
        state.currentTime >= targetSegment.sourceStart &&
        state.currentTime <= targetSegment.sourceEnd
      ) {
        nextCurrentTime = clamp(
          state.currentTime,
          resizedSegment.sourceStart,
          Math.max(resizedSegment.sourceStart, resizedSegment.sourceEnd - 0.001),
        );
      }

      return {
        ...state,
        segments: nextSegments,
        currentTime: nextCurrentTime,
        selectedSegmentId: id,
      };
    });

    refreshPreviewPlaybackAfterSegmentEdit();

  }

  function splitAtRangeMarkers(): void {
    if (!canUseRange || !normalizedRange) {
      return;
    }

    pushUndoSnapshot();
    splitAtTime(normalizedRange.start, { recordUndo: false, toast: false });
    splitAtTime(normalizedRange.end, { recordUndo: false, toast: false });
    noteEdit('Cut at In and Out.');
    refreshPreviewPlaybackAfterSegmentEdit();
  }

  function keepOnlyRange(): void {
    if (!canUseRange || !normalizedRange) {
      return;
    }

    pushUndoSnapshot();
    const result = keepOnlyTimelineRange(normalizedRange);
    editor.update((state) => ({
      ...state,
      segments: result.segments,
      selectedSegmentId: result.selectedSegmentId,
    }));
    noteEdit(`Kept only ${formatTime(normalizedRange.start)} – ${formatTime(normalizedRange.end)}.`);
    seekTo(normalizedRange.start);
  }

  function deleteOutsideRange(): void {
    if (!canUseRange || !normalizedRange) {
      return;
    }

    const { start, end } = normalizedRange;
    const result = deleteOutsideTimelineRange($editor.segments, { start, end });
    pushUndoSnapshot();
    editor.update((state) => ({
      ...state,
      segments: result.segments,
      selectedSegmentId: result.selectedSegmentId,
    }));
    noteEdit('Removed everything outside In and Out.');
    refreshPreviewPlaybackAfterSegmentEdit();
  }

  function toggleRangeLoop(): void {
    if (!canUseRange) {
      return;
    }

    rangeLoopPlayback = !rangeLoopPlayback;
    rangeLoopUserSet = true;

    if (rangeLoopPlayback && normalizedRange) {
      seekTo(normalizedRange.start);
    } else if (sequencePlayback.isPlaying()) {
      sequencePlayback.onSegmentsChanged();
    }
  }

  function zoomToRange(): void {
    if (!canUseRange || !normalizedRange) {
      return;
    }

    timeline?.zoomToSourceRange(normalizedRange.start, normalizedRange.end);
  }

  function setRangeMarker(marker: 'start' | 'end', seconds: number): void {
    const result = setTimelineRangeMarker(marker, seconds, duration, { rangeStart, rangeEnd });
    rangeStart = result.rangeStart;
    rangeEnd = result.rangeEnd;
    loopNarrowedRange();
  }

  // Trimming is about timing: once the range is narrower than the clip, loop it
  // so the cut can be felt. Only until the user turns loop on or off themselves.
  function loopNarrowedRange(): void {
    if (rangeLoopUserSet || rangeStart === null || rangeEnd === null) {
      return;
    }
    const narrowed = Math.abs(rangeEnd - rangeStart) < duration - 0.05;
    if (narrowed && !rangeLoopPlayback) {
      rangeLoopPlayback = true;
    }
  }

  function clearRange(): void {
    const result = clearTimelineRange();
    rangeStart = result.rangeStart;
    rangeEnd = result.rangeEnd;
  }

  function defaultBookmarkLabel(): string {
    return `Marker ${bookmarks.length + 1}`;
  }

  function addBookmarkMarker(seconds: number, label?: string): void {
    if (!metadata) {
      return;
    }

    const result = addTimelineBookmark(bookmarks, seconds, metadata.duration, {
      label,
      defaultLabel: defaultBookmarkLabel(),
      dedupeSeconds: BOOKMARK_DEDUPE_SECONDS,
    });
    if (result.status === 'duplicate') {
      noteEdit('A marker already exists at this time.');
      return;
    }

    bookmarks = result.bookmarks;
    selectedBookmarkId = result.selectedBookmarkId;
  }

  function updateBookmarkLabel(id: string, label: string): void {
    bookmarks = updateTimelineBookmarkLabel(bookmarks, id, label);
  }

  function removeBookmark(id: string): void {
    const result = removeTimelineBookmark(bookmarks, id, selectedBookmarkId);
    bookmarks = result.bookmarks;
    selectedBookmarkId = result.selectedBookmarkId;
  }

  function deleteSelectedBookmark(): void {
    if (!selectedBookmarkId) {
      return;
    }

    removeBookmark(selectedBookmarkId);
    noteEdit('Marker deleted.');
  }

  function openBookmarkLabelEditor(id: string): void {
    if (!bookmarks.some((bookmark) => bookmark.id === id)) {
      return;
    }

    editingBookmarkId = id;
    markerLabelModalOpen = true;
  }

  function goToNextMarker(): void {
    if (!bookmarks.length) {
      return;
    }

    const current = $editor.currentTime;
    const next = bookmarks.find((bookmark) => bookmark.time > current + 0.001);
    const target = next ?? bookmarks[0];
    selectedBookmarkId = target.id;
    seekTo(target.time);
  }

  function goToPreviousMarker(): void {
    if (!bookmarks.length) {
      return;
    }

    const current = $editor.currentTime;
    const previous = [...bookmarks].reverse().find((bookmark) => bookmark.time < current - 0.001);
    const target = previous ?? bookmarks[bookmarks.length - 1];
    selectedBookmarkId = target.id;
    seekTo(target.time);
  }

  function cancelPendingWaveform(): void {
    pendingWaveform = null;

    if (waveformDeferTimer) {
      clearTimeout(waveformDeferTimer);
      waveformDeferTimer = null;
    }
  }

  function queueWaveformAfterPreview(path: string, hasAudio: boolean, duration: number): void {
    cancelPendingWaveform();
    pendingWaveform = {
      path,
      hasAudio,
      generation: waveformLoadGeneration,
      duration,
    };

    waveformDeferTimer = setTimeout(() => {
      waveformDeferTimer = null;
      startPendingWaveform();
    }, 12_000);

    void tick().then(() => {
      if (preview?.isReady()) {
        handlePreviewReady();
      }
    });
  }

  async function loadThumbnails(path: string, clipSeconds: number): Promise<void> {
    const generation = ++thumbnailGeneration;
    const count = thumbnailCount(clipSeconds);
    const times = thumbnailTimes(clipSeconds, count);
    const order = coarseToFineOrder(count);
    thumbnailsFor = path;
    thumbnails = new Array(count).fill(null);

    let next = 0;
    const worker = async (): Promise<void> => {
      while (next < order.length && generation === thumbnailGeneration) {
        const index = order[next++];
        try {
          const frame = await invoke<string | null>('extract_thumbnail', { path, time: times[index], height: 72 });
          if (generation !== thumbnailGeneration) {
            return;
          }
          if (frame) {
            thumbnails[index] = frame;
            thumbnails = thumbnails;
          }
        } catch {
          // A missing frame just leaves that tile empty.
        }
      }
    };
    await Promise.all([worker(), worker(), worker()]);
  }

  function startPendingWaveform(): void {
    const pending = pendingWaveform;

    if (!pending || pending.generation !== waveformLoadGeneration) {
      return;
    }

    pendingWaveform = null;

    if (waveformDeferTimer) {
      clearTimeout(waveformDeferTimer);
      waveformDeferTimer = null;
    }

    void loadWaveform(pending.path, pending.hasAudio, pending.generation, pending.duration);
  }

  function handlePreviewReady(): void {
    if (autoplayOnReady) {
      autoplayOnReady = false;
      preview?.playPlayback();
    }

    // Thumbnails come from the original file (ffmpeg reads any codec), once per clip.
    if ($editor.currentFile && metadata && thumbnailsFor !== $editor.currentFile) {
      void loadThumbnails($editor.currentFile, metadata.duration);
    }

    if (!pendingWaveform || pendingWaveform.generation !== waveformLoadGeneration) {
      return;
    }

    if (waveformDeferTimer) {
      clearTimeout(waveformDeferTimer);
      waveformDeferTimer = null;
    }

    setTimeout(() => {
      startPendingWaveform();
    }, 200);
  }

  async function loadWaveform(
    path: string,
    hasAudio: boolean,
    generation: number,
    duration: number,
  ): Promise<void> {
    if (generation !== waveformLoadGeneration) {
      return;
    }

    waveformLoading = true;
    waveformPeaks = [];

    if (!hasAudio) {
      waveformLoading = false;
      return;
    }

    try {
      const peaks = await invoke<number[]>('extract_waveform', {
        path,
        bucketCount: 2400,
        hasAudio: true,
        duration,
      });

      if (generation !== waveformLoadGeneration) {
        return;
      }

      waveformPeaks = peaks;
    } catch {
      if (generation === waveformLoadGeneration) {
        waveformPeaks = [];
      }
    } finally {
      if (generation === waveformLoadGeneration) {
        waveformLoading = false;
      }
    }
  }

  function removeNearestBookmark(seconds: number): boolean {
    if (!bookmarks.length) {
      return false;
    }

    let nearest = bookmarks[0];
    let nearestDistance = Math.abs(nearest.time - seconds);

    for (const bookmark of bookmarks) {
      const distance = Math.abs(bookmark.time - seconds);
      if (distance < nearestDistance) {
        nearest = bookmark;
        nearestDistance = distance;
      }
    }

    if (nearestDistance > BOOKMARK_REMOVE_TOLERANCE) {
      return false;
    }

    removeBookmark(nearest.id);
    return true;
  }

  function snapToRangeStart(): void {
    if (!normalizedRange) {
      return;
    }

    seekTo(normalizedRange.start);
  }

  function snapToRangeEnd(): void {
    if (!normalizedRange) {
      return;
    }

    seekTo(normalizedRange.end);
  }

  function updateTrackHeights(event: CustomEvent<{ videoHeight: number; audioHeight: number }>): void {
    videoTrackHeight = event.detail.videoHeight;
    audioTrackHeight = event.detail.audioHeight;
  }

  function zoomToFit(): void {
    timeline?.zoomToFitView();
  }

  function closeModals(): void {
    exportModalOpen = false;
    exportPanelOpen = false;
    settingsModalOpen = false;
    shortcutsModalOpen = false;
    historyDrawerOpen = false;
    markerLabelModalOpen = false;
    uploadTargetModalOpen = false;
    watchFolderConfirmOpen = false;
    clearHistoryConfirmOpen = false;
    relinkModalOpen = false;
    editingBookmarkId = null;
  }

  function resolveUploadProvidersFromSettings(settings: AppSettings): UploadProvider[] {
    const settingsRecord = settings as AppSettings & Record<string, unknown>;
    let providers = readUploadProvidersFromAppSettings(settingsRecord);
    if (providers.length > 0) {
      return providers;
    }

    const hasLegacyCatbox =
      Boolean(settings.catboxUserHash?.trim()) || Boolean(settings.catboxApiUrl?.trim());
    if (hasLegacyCatbox) {
      return [
        {
          ...createCatboxProvider(),
          config: {
            apiUrl: settings.catboxApiUrl?.trim() || 'https://catbox.moe/user/api.php',
            userHash: settings.catboxUserHash?.trim() || null,
          },
        },
      ];
    }

    return [createFilegardenProvider()];
  }

  function applySettingsFromDisk(settings: AppSettings): void {
    const settingsRecord = settings as AppSettings & Record<string, unknown>;
    watchFolder = settings.watchFolder;
    watchFolderEnabled = settings.watchFolderEnabled;
    defaultExportDir = settings.defaultExportDir ?? settings.lastExportDir;
    exportPresetId = settings.lastPresetId || 'lossless-trim';
    preferGpuEncoding = settings.preferGpuEncoding;
    runAtStartup = settings.runAtStartup;
    startMinimizedToTray = settings.startMinimizedToTray;
    closeToTray = settings.closeToTray ?? true;
    uploadProviders = resolveUploadProvidersFromSettings(settings);
    defaultUploadProviderId =
      readDefaultUploadProviderId(settingsRecord) ??
      uploadProviders.find((provider) => provider.enabled)?.id ??
      null;
    customExportPresets = parseCustomPresetsFromSettings(settings.customExportPresets);
    syncUploadProviderSelection();
  }

  async function closeSettingsModal(): Promise<void> {
    settingsModalOpen = false;
    try {
      const settings = await invoke<AppSettings>('get_settings');
      applySettingsFromDisk(settings);
    } catch {
      // Parent state was not mutated by the modal draft; ignore reload errors on close.
    }
  }

  async function refreshExportPresets(): Promise<void> {
    exportPresets = await invoke<PresetInfo[]>('list_presets');
  }

  async function openSettings(tab: 'general' | 'folders' | 'presets' | 'upload' = 'general'): Promise<void> {
    settingsInitialTab = tab;
    try {
      const settings = await invoke<AppSettings>('get_settings');
      applySettingsFromDisk(settings);
      settingsModalOpen = true;
    } catch (error) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: error instanceof Error ? error.message : String(error),
        },
      }));
    }
  }

  function exportCropPixels(): { x: number; y: number; width: number; height: number } | null {
    if (!cropEnabled || !metadata) {
      return null;
    }

    const even = (value: number) => value - (value % 2);
    return {
      x: even(Math.round(cropRect.x * metadata.width)),
      y: even(Math.round(cropRect.y * metadata.height)),
      width: even(Math.round(cropRect.width * metadata.width)),
      height: even(Math.round(cropRect.height * metadata.height)),
    };
  }

  function applyAspectCrop(aspect: 'free' | '16:9' | '9:16' | '4:3' | '1:1' | 'custom'): void {
    cropAspect = aspect;

    if (aspect === 'free') {
      cropLockAspect = false;
      return;
    }

    cropLockAspect = true;

    if (!metadata) {
      return;
    }

    const frameRatio = metadata.width / metadata.height;
    let targetRatio = 16 / 9;
    if (aspect === '9:16') {
      targetRatio = 9 / 16;
    } else if (aspect === '4:3') {
      targetRatio = 4 / 3;
    } else if (aspect === '1:1') {
      targetRatio = 1;
    } else if (aspect === 'custom') {
      targetRatio = Math.max(1, cropCustomAspectW) / Math.max(1, cropCustomAspectH);
    }
    let width = 1;
    let height = 1;

    if (frameRatio > targetRatio) {
      width = targetRatio / frameRatio;
      height = 1;
    } else {
      width = 1;
      height = frameRatio / targetRatio;
    }

    cropRect = {
      x: (1 - width) / 2,
      y: (1 - height) / 2,
      width,
      height,
    };
  }

  async function refreshClipHistory(): Promise<void> {
    clipHistory = await invoke<ClipHistoryEntry[]>('list_clip_history');
  }

  async function removeHistoryEntry(outputPath: string): Promise<void> {
    clipHistory = await invoke<ClipHistoryEntry[]>('remove_clip_history_entry', { outputPath });
    noteEdit('Removed from clip history.');
  }

  function requestClearHistory(): void {
    if (clipHistory.length === 0) {
      return;
    }
    clearHistoryConfirmOpen = true;
  }

  async function clearHistory(): Promise<void> {
    clearHistoryConfirmOpen = false;
    await invoke('clear_clip_history');
    clipHistory = [];
    noteEdit('Clip history cleared.');
  }

  function uploadProviderName(providerId: string | null): string {
    const summary = uploadProviderSummaries.find((entry) => entry.id === providerId);
    if (summary) {
      return summary.name;
    }

    const provider = uploadProviders.find((entry) => entry.id === providerId);
    return provider?.name ?? 'upload host';
  }

  async function refreshUploadProviderSummaries(): Promise<void> {
    const summaries = await invoke<UploadProviderSummary[]>('list_upload_providers');
    uploadProviderSummaries = normalizeUploadSummaries(summaries);
    syncUploadProviderSelection();
  }

  function getEnabledUploadTargets(): UploadProviderSummary[] {
    const fromSummaries = uploadProviderSummaries.filter((entry) => entry.enabled);
    if (fromSummaries.length > 0) {
      return fromSummaries;
    }

    return uploadProviders
      .filter((provider) => provider.enabled)
      .map((provider) => ({
        id: provider.id,
        name: provider.name,
        kind: provider.kind,
        enabled: true,
        isDefault: provider.id === defaultUploadProviderId,
      }));
  }

  function syncUploadProviderSelection(): void {
    const enabled = getEnabledUploadTargets();
    if (enabled.length === 0) {
      selectedUploadProviderId = null;
      return;
    }

    if (
      defaultUploadProviderId &&
      !enabled.some((entry) => entry.id === defaultUploadProviderId)
    ) {
      defaultUploadProviderId = enabled[0]?.id ?? null;
    }

    if (
      !selectedUploadProviderId ||
      !enabled.some((entry) => entry.id === selectedUploadProviderId)
    ) {
      selectedUploadProviderId =
        defaultUploadProviderId &&
        enabled.some((entry) => entry.id === defaultUploadProviderId)
          ? defaultUploadProviderId
          : enabled[0].id;
    }
  }

  $: uploadProviders, defaultUploadProviderId, syncUploadProviderSelection();

  function resolveUploadProviderId(providerId?: string | null): string | null {
    const enabled = getEnabledUploadTargets();
    const candidates = [providerId, selectedUploadProviderId, defaultUploadProviderId];
    for (const id of candidates) {
      if (id && enabled.some((entry) => entry.id === id)) {
        return id;
      }
    }

    const fallback = enabled.find((entry) => entry.isDefault) ?? enabled[0];
    return fallback?.id ?? null;
  }

  async function uploadClip(path: string, providerId?: string | null): Promise<void> {
    const enabled = getEnabledUploadTargets();
    if (enabled.length === 0) {
      pushToast('No upload targets configured. Add one in Settings.', 'error');
      settingsModalOpen = true;
      return;
    }

    if (enabled.length > 1 && providerId === undefined) {
      openUploadPicker(path);
      return;
    }

    const targetProviderId = resolveUploadProviderId(providerId);
    if (!targetProviderId) {
      openUploadPicker(path);
      return;
    }

    selectedUploadProviderId = targetProviderId;
    const providerName = uploadProviderName(targetProviderId);
    historyBusyPath = path;
    statusDismissed = false;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message: `Uploading to ${providerName}...`,
        outputPath: state.exportStatus.outputPath ?? path,
        outputSize: state.exportStatus.outputSize,
      },
    }));

    try {
      const url = await invoke<string>('upload_file', {
        filePath: path,
        providerId: targetProviderId,
      });
      await invoke('copy_text_to_clipboard', { text: url });
      lastShareUrl = url;
      clipHistory = await invoke<ClipHistoryEntry[]>('update_clip_history_share_url', {
        outputPath: path,
        shareUrl: url,
      });
      await refreshUploadProviderSummaries();
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'success',
          message: `Uploaded to ${providerName}. Link copied.`,
          outputPath: state.exportStatus.outputPath ?? path,
          outputSize: state.exportStatus.outputSize,
        },
      }));
      pushToast('Upload complete. Link copied to clipboard.', 'success');
      await notify('Upload link copied to clipboard.', `${providerName} upload`);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message,
          outputPath: state.exportStatus.outputPath,
          outputSize: state.exportStatus.outputSize,
        },
      }));
      pushToast(message, 'error');
    } finally {
      historyBusyPath = null;
      uploadTargetModalOpen = false;
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    const action = resolveShortcut(event, {
      isTextInput:
        target?.tagName === 'TEXTAREA' ||
        Boolean(target?.isContentEditable) ||
        (target instanceof HTMLInputElement && !['button', 'checkbox', 'radio', 'range'].includes(target.type)),
      modals: {
        export: exportModalOpen,
        settings: settingsModalOpen,
        history: historyDrawerOpen,
        shortcuts: shortcutsModalOpen,
        markerLabel: markerLabelModalOpen,
        exportPanel: exportPanelOpen,
      },
      hasSelectedSegment: Boolean($editor.selectedSegmentId),
      hasClip: canExport,
      hasRange: canUseRange && Boolean(normalizedRange),
    });

    if (!action) {
      return;
    }

    event.preventDefault();
    runShortcut(action, event.shiftKey);
  }

  const SHUTTLE_RATES = [0.5, 1, 1.5, 2];

  // J/K/L as in editors. Browsers can't play backwards, so J slows down and,
  // once paused, steps back a second (hold to keep stepping).
  function shuttle(direction: 1 | -1): void {
    const index = SHUTTLE_RATES.indexOf(previewPlaybackRate);
    if (!previewPlaying) {
      if (direction > 0) {
        previewPlaybackRate = 1;
        preview?.playPlayback();
      } else {
        seekBySequence(-1);
      }
      return;
    }
    if (direction < 0 && previewPlaybackRate <= SHUTTLE_RATES[0]) {
      preview?.pausePlayback();
      previewPlaybackRate = 1;
      return;
    }
    const next = clamp((index < 0 ? 1 : index) + direction, 0, SHUTTLE_RATES.length - 1);
    previewPlaybackRate = SHUTTLE_RATES[next];
  }

  function runShortcut(action: ShortcutAction, shiftKey: boolean): void {
    // Step by the highest frame rate so variable-frame-rate clips never skip a frame.
    const stepFps = metadata?.maxFps || metadata?.fps || 30;
    const frameStep = 1 / stepFps;
    const step = shiftKey ? 1 : frameStep;

    switch (action) {
      case 'closeModals':
        return closeModals();
      case 'deselect':
        return deselectSegment();
      case 'showHelp':
        shortcutsModalOpen = true;
        return;
      case 'undo':
        return undoSegmentEdit();
      case 'redo':
        return redoSegmentEdit();
      case 'duplicate':
        return duplicateSelectedSegment();
      case 'open':
        void chooseClip();
        return;
      case 'save':
        if ($editor.currentFile) {
          void saveProject();
        } else {
          pushToast('Open a clip before saving a project.', 'info');
        }
        return;
      case 'export':
        if (canExport && !openingClip) {
          toggleExportPanel();
        } else {
          pushToast('Open a clip before exporting.', 'info');
        }
        return;
      case 'markIn':
        return setRangeMarker('start', $editor.currentTime);
      case 'markOut':
        return setRangeMarker('end', $editor.currentTime);
      case 'togglePlay':
        return preview?.togglePlayback();
      case 'shuttleBack':
        return shuttle(-1);
      case 'pause':
        previewPlaybackRate = 1;
        return preview?.pausePlayback();
      case 'shuttleForward':
        return shuttle(1);
      case 'toRangeStart':
        if (normalizedRange) seekTo(normalizedRange.start);
        return;
      case 'toRangeEnd':
        if (normalizedRange) seekTo(normalizedRange.end);
        return;
      case 'previousMarker':
        return goToPreviousMarker();
      case 'nextMarker':
        return goToNextMarker();
      case 'removeMarkerAtPlayhead':
        if (removeNearestBookmark($editor.currentTime)) {
          noteEdit('Removed marker at playhead.');
        }
        return;
      case 'addMarker':
        return addBookmarkMarker($editor.currentTime);
      case 'split':
        return splitAtCurrentTime();
      case 'delete':
        if (selectedBookmarkId) {
          deleteSelectedBookmark();
        } else {
          deleteSelectedSegment();
        }
        return;
      case 'toggleRangeLoop':
        return toggleRangeLoop();
      case 'zoomToRange':
        return zoomToRange();
      case 'stepBack':
        return seekBySequence(-step);
      case 'stepForward':
        return seekBySequence(step);
    }
  }

  function defaultExportFileName(): string {
    const extension = extensionForExport(exportType, audioPresetId);
    if (!$editor.currentFile) {
      return `cutdown.${extension}`;
    }

    const leaf = $editor.currentFile.split(/[\\/]/).pop() ?? 'clip.mp4';
    const stem = leaf.replace(/(\.[^.]+)?$/i, '-cutdown');
    return `${stem}.${extension}`;
  }

  function defaultExportDirectory(settings?: AppSettings | null): string {
    return (
      defaultExportDir ??
      settings?.defaultExportDir ??
      settings?.lastExportDir ??
      $editor.currentFile?.replace(/[\\/][^\\/]+$/, '') ??
      ''
    );
  }

  function defaultOutputPath(): string {
    return joinOutputPath(
      defaultExportDirectory(),
      defaultExportFileName(),
      extensionForExport(exportType, audioPresetId),
    );
  }

  function syncExportDefaultsForClip(settings?: AppSettings | null): void {
    outputFileName = defaultExportFileName();
    outputDirectory = defaultExportDirectory(settings);
  }

  async function chooseOutput(): Promise<void> {
    const settings = await invoke<AppSettings>('get_settings');
    const defaultName = defaultExportFileName();
    const defaultDir = defaultExportDirectory(settings) || undefined;
    const extension = extensionForExport(exportType, audioPresetId);
    const saveFilters =
      exportType === 'audio'
        ? [
            { name: 'WAV audio', extensions: ['wav'] },
            { name: 'MP3 audio', extensions: ['mp3'] },
            { name: 'OGG audio', extensions: ['ogg'] },
          ]
        : [{ name: 'MP4 Video', extensions: ['mp4'] }];

    const selected = await save({
      defaultPath:
        outputPath ||
        (outputDirectory
          ? joinOutputPath(outputDirectory, outputFileName, extension)
          : defaultDir
            ? joinOutputPath(defaultDir, defaultName, extension)
            : defaultOutputPath()),
      filters: saveFilters,
    });

    if (selected) {
      applyOutputPath(selected);
      if (exportType === 'audio') {
        const { fileName } = splitOutputPath(selected);
        const parsed = fileName.match(/\.([^.]+)$/i)?.[1]?.toLowerCase();
        if (parsed === 'wav') {
          audioPresetId = 'audio-wav';
        } else if (parsed === 'ogg') {
          audioPresetId = 'audio-ogg-192';
        } else if (parsed === 'mp3') {
          audioPresetId = DEFAULT_AUDIO_PRESET_ID;
        }
      }
      if (outputDirectory) {
        await invoke('set_last_export_dir', { path: outputDirectory });
      }
    }
  }

  async function openExportModal(): Promise<void> {
    const settings = await invoke<AppSettings>('get_settings').catch(() => null);
    outputFileName = ensureExportFileExtension(
      defaultExportFileName(),
      extensionForExport(exportType, audioPresetId),
    );

    if (!outputDirectory) {
      outputDirectory = defaultExportDirectory(settings);
    }

    exportModalOpen = true;
  }

  function buildExportSegmentRanges(): Array<{ start: number; end: number }> {
    if (exportMode === 'range' && canUseRange && normalizedRange) {
      return [{ start: normalizedRange.start, end: normalizedRange.end }];
    }

    return segments.map((segment) => ({
      start: segment.sourceStart,
      end: segment.sourceEnd,
    }));
  }

  function buildQueuedExportJobs(): ExportJob[] {
    const exportSegments = buildExportSegmentRanges();
    if (exportSegments.length === 0) {
      return [];
    }

    const extension = extensionForExport(exportType, audioPresetId);

    if (exportBatchPerSegment && exportMode === 'sequence' && exportSegments.length > 1) {
      return buildPerSegmentJobs(outputDirectory, outputFileName, exportSegments, extension);
    }

    return [
      {
        id: crypto.randomUUID(),
        outputPath,
        segments: exportSegments,
        label: 'Export',
      },
    ];
  }

  async function confirmOverwrite(paths: string[]): Promise<boolean> {
    for (const path of paths) {
      const exists = await invoke<boolean>('path_exists', { path }).catch(() => false);
      if (!exists) {
        continue;
      }

      const overwrite = await confirm(`Replace existing file?\n${path}`, {
        title: 'Overwrite output',
        kind: 'warning',
      });

      if (!overwrite) {
        return false;
      }
    }

    return true;
  }

  async function notifyExportComplete(title: string, body: string): Promise<void> {
    try {
      const visible = await getCurrentWindow().isVisible();
      if (!visible) {
        await notify(body, title);
      }
    } catch {
      // Notification while minimized is best-effort.
    }
  }

  async function runExportJob(job: ExportJob): Promise<ExportResult> {
    const preset = exportPresets.find((item) => item.id === exportPresetId);
    const presetLabel =
      exportType === 'audio' ? audioPresetLabel(audioPresetId) : (preset?.name ?? exportPresetId);
    const exportKindLabel = exportType === 'audio' ? 'audio' : job.label;

    exportProgressPercent = 0;
    exportStartedAt = Date.now();
    statusDismissed = false;
    exportJobLabel = job.label;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message:
          exportType === 'audio'
            ? `Exporting ${exportKindLabel} as ${presetLabel}...`
            : `Exporting ${job.label} with ${presetLabel}...`,
        outputPath: job.outputPath,
      },
    }));

    const crop = exportType === 'audio' ? null : exportCropPixels();
    const result = await invoke<ExportResult>('export_clip', {
      params: {
        inputPath: $editor.currentFile,
        outputPath: job.outputPath,
        audioMode: stripAudioExport ? 'strip' : 'preserve',
        segments: job.segments,
        presetId: exportPresetId,
        preferGpu: preferGpuEncoding,
        sourcePath: $editor.currentFile,
        crop,
        volume: clipVolume,
        accurateTrim: accurateTrimExport,
        fadeInSeconds: fadeInSeconds > 0 ? fadeInSeconds : null,
        fadeOutSeconds: fadeOutSeconds > 0 ? fadeOutSeconds : null,
        exportKind: exportType,
        audioPresetId: exportType === 'audio' ? audioPresetId : null,
        audioFormat: exportType === 'audio' ? audioPresetFormat(audioPresetId) : null,
      },
    });

    exportProgressPercent = 100;
    lastShareUrl = null;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'success',
        message:
          exportJobTotal > 1
            ? `Exported ${job.label}: ${formatBytes(result.fileSize)}.`
            : `Saved ${formatBytes(result.fileSize)}, ${formatTime(result.duration)} long. Copy file to paste it into Discord.`,
        outputPath: result.outputPath,
        outputSize: result.fileSize,
      },
    }));
    pushToast(`Exported ${formatBytes(result.fileSize)} clip.`, 'success');

    await notifyExportComplete('Export complete', `Exported ${formatBytes(result.fileSize)} clip.`);

    if (queueUploadAfterExport) {
      await uploadClip(result.outputPath, selectedUploadProviderId);
    }

    return result;
  }

  async function processExportQueue(jobs: ExportJob[]): Promise<void> {
    if (exportQueueProcessing || jobs.length === 0) {
      return;
    }

    exportQueueProcessing = true;
    exportJobTotal = jobs.length;
    exportJobIndex = 0;

    try {
      for (let index = 0; index < jobs.length; index += 1) {
        exportJobIndex = index + 1;
        await runExportJob(jobs[index]);
      }

      await refreshClipHistory();
      if (exportType === 'video') {
        await invoke('set_last_preset_id', { presetId: exportPresetId });
      }
      const parent = jobs[jobs.length - 1]?.outputPath.replace(/[\\/][^\\/]+$/, '');
      if (parent) {
        await invoke('set_last_export_dir', { path: parent });
      }
    } catch (error) {
      exportProgressPercent = null;
      const message = error instanceof Error ? error.message : String(error);
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message,
          outputPath: state.exportStatus.outputPath,
          outputSize: state.exportStatus.outputSize,
        },
      }));
      pushToast(message, 'error');
    } finally {
      exportQueueProcessing = false;
      exportJobIndex = 1;
      exportJobTotal = 1;
      exportJobLabel = '';
    }
  }

  function toggleExportPanel(): void {
    if (!canExport || openingClip || exportQueueProcessing) {
      exportPanelOpen = false;
      return;
    }
    exportPanelOpen = !exportPanelOpen;
  }

  function openFullExportOptions(): void {
    exportPanelOpen = false;
    void openExportModal();
  }

  // One-click export: kept parts trimmed to In/Out, saved next to the original with a
  // readable name. No save dialog; never overwrites (adds " (2)" instead).
  async function quickExport(): Promise<void> {
    if (!$editor.currentFile || quickRanges.length === 0) {
      return;
    }
    if (!ffmpegAvailable) {
      pushToast(ffmpegStatus || 'ffmpeg is needed to export. Install it from the banner at the top.', 'error');
      return;
    }

    exportType = 'video';
    const extension = extensionForExport('video', audioPresetId);
    let path = quickExportPath($editor.currentFile, quickRanges[0].start, extension);
    for (let attempt = 2; await invoke<boolean>('path_exists', { path }).catch(() => false); attempt += 1) {
      path = quickExportPath($editor.currentFile, quickRanges[0].start, extension, attempt);
    }

    exportPanelOpen = false;
    await processExportQueue([{ id: crypto.randomUUID(), outputPath: path, segments: quickRanges, label: 'Export' }]);
  }

  async function copyExportedFile(): Promise<void> {
    const path = $editor.exportStatus.outputPath;
    if (!path) {
      return;
    }
    try {
      await invoke('copy_file_to_clipboard', { path });
      noteEdit('File copied. Paste it into Discord or a folder with Ctrl+V.');
    } catch (error) {
      pushToast(error instanceof Error ? error.message : String(error), 'error');
    }
  }

  async function exportClip(): Promise<void> {
    const extension = extensionForExport(exportType, audioPresetId);
    outputFileName = ensureExportFileExtension(outputFileName, extension);

    if (!$editor.currentFile || !outputDirectory) {
      return;
    }

    if (!ffmpegAvailable) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: ffmpegStatus || 'ffmpeg is not available.',
        },
      }));
      return;
    }

    const jobs = buildQueuedExportJobs();
    if (jobs.length === 0) {
      return;
    }

    if (!(await confirmOverwrite(jobs.map((job) => job.outputPath)))) {
      return;
    }

    exportModalOpen = false;
    await processExportQueue(jobs);
  }

  async function prepareProxyPreview(): Promise<void> {
    if (!$editor.currentFile || previewFallbackRunning) {
      return;
    }

    previewFallbackRunning = true;
    exportProgressPercent = null;
    openingMessage = 'Building a playable preview…';
    editor.update((state) => ({
      ...state,
      exportStatus: { state: 'running', message: 'Making a preview copy…' },
    }));

    try {
      const result = await invoke<PreviewResult>('prepare_preview', {
        params: { inputPath: $editor.currentFile, forceProxy: true },
      });
      await cleanupPreview($editor.previewTempPath);
      editor.update((state) => ({
        ...state,
        videoSrc: mediaSrcFromPath(result.previewPath),
        previewTempPath: result.previewPath,
        previewStrategy: result.strategy as EditorState['previewStrategy'],
        exportStatus: {
          state: 'idle',
          message: `Preview copy ready. ${READY_HINT}`,
        },
      }));
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: detail,
        },
      }));
      pushToast(detail, 'error');
    } finally {
      previewFallbackRunning = false;
    }
  }

  async function loadLatestReplay(): Promise<void> {
    let folder = watchFolder;
    if (!folder) {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Choose replay folder',
      });
      if (typeof selected !== 'string') {
        pushToast('Set a watch folder in Settings → Folders to use Latest replay.', 'info');
        return;
      }

      folder = selected;
      const saved = await invoke<AppSettings>('update_watch_folder', {
        path: folder,
        enabled: true,
      });
      watchFolder = saved.watchFolder;
      watchFolderEnabled = saved.watchFolderEnabled;
    }

    try {
      const latest = await invoke<{ path: string | null; message: string }>(
        'find_latest_replay_in_folder',
        { folder },
      );
      if (latest.path) {
        await openClipPath(latest.path);
        return;
      }

      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: latest.message,
        },
      }));
      pushToast(latest.message, 'error');
    } catch (error) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: error instanceof Error ? error.message : String(error),
        },
      }));
    }
  }

  function currentProjectPayload(): CutdownProject {
    return createCutdownProjectPayload({
      sourcePath: $editor.currentFile ?? '',
      segments: $editor.segments,
      selectedSegmentId: $editor.selectedSegmentId,
      rangeStart,
      rangeEnd,
      cropEnabled,
      cropRect,
      clipVolume,
      currentTime: $editor.currentTime,
      bookmarks,
      exportPresetId,
      accurateTrim: accurateTrimExport,
      stripAudio: stripAudioExport,
      exportType,
      audioPresetId,
    });
  }

  async function saveProject(): Promise<void> {
    if (!$editor.currentFile) {
      return;
    }

    const selected = await save({
      defaultPath: $editor.currentFile.replace(/\.[^.]+$/i, `.${CUTDOWN_PROJECT_EXTENSION}`),
      filters: [{ name: 'Cutdown project', extensions: [CUTDOWN_PROJECT_EXTENSION] }],
    });

    if (!selected) {
      return;
    }

    await invoke('save_project_file', {
      path: selected,
      project: currentProjectPayload(),
    });

    editor.update((state) => ({
      ...state,
      exportStatus: { state: 'idle', message: `Project saved to ${selected}.` },
    }));
  }

  async function openProject(): Promise<void> {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Cutdown project', extensions: [CUTDOWN_PROJECT_EXTENSION] }],
    });

    if (!selected || typeof selected !== 'string') {
      return;
    }

    await openIncomingPath(selected);
  }

  async function openProjectPath(selected: string): Promise<void> {
    const project = await invoke<CutdownProject>('load_project_file', { path: selected });

    if (!(await invoke<boolean>('path_exists', { path: project.sourcePath }))) {
      pendingProject = project;
      relinkMissingPath = project.sourcePath;
      relinkProjectName = selected.split(/[\\/]/).pop() ?? selected;
      relinkModalOpen = true;
      return;
    }

    await applyLoadedProject(project, selected);
  }

  async function applyLoadedProject(project: CutdownProject, projectLabel: string): Promise<void> {
    await openClipPath(project.sourcePath);
    rangeStart = project.rangeStart;
    rangeEnd = project.rangeEnd;
    bookmarks = (project.bookmarks ?? []).map((bookmark) => ({
      id: bookmark.id,
      time: bookmark.time,
      label: bookmark.label,
    }));
    clipVolume = clamp(project.clipVolume ?? 1, 0, 1);
    cropEnabled = project.cropEnabled;
    cropRect = project.cropRect;
    accurateTrimExport = project.accurateTrim;
    stripAudioExport = project.stripAudio;
    if (project.exportPresetId) {
      exportPresetId = project.exportPresetId;
    }
    exportType = parseExportType(project.exportType) ?? 'video';
    audioPresetId = project.audioPresetId
      ? resolveAudioPresetId(project.audioPresetId)
      : migrateLegacyAudioFormat(project.audioFormat);

    editor.update((state) => ({
      ...state,
      segments: project.segments,
      selectedSegmentId: project.selectedSegmentId,
      currentTime: clamp(project.currentTime ?? 0, 0, state.metadata?.duration ?? 0),
      exportStatus: { state: 'idle', message: `Loaded project ${projectLabel}.` },
    }));
    await tick();
    seekTo(get(editor).currentTime);
    noteEdit(`Loaded project ${projectLabel}.`);
  }

  async function relinkProjectSource(): Promise<void> {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Video',
          extensions: [...VIDEO_EXTENSIONS],
        },
      ],
    });

    if (!selected || typeof selected !== 'string' || !pendingProject) {
      return;
    }

    relinkModalOpen = false;
    const project = { ...pendingProject, sourcePath: selected };
    pendingProject = null;
    await applyLoadedProject(project, relinkProjectName);
    relinkMissingPath = '';
    relinkProjectName = '';
  }

  async function handlePreviewError(message: string): Promise<void> {
    if (!$editor.currentFile || previewFallbackRunning) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message,
        },
      }));
      pushToast(message, 'error');
      return;
    }

    if ($editor.previewStrategy === 'Preview proxy') {
      const detail = `The preview can’t play this file (${message}). You can still cut and export: export reads the original, not the preview.`;
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: detail,
        },
      }));
      pushToast(detail, 'error');
      return;
    }

    previewFallbackRunning = true;
    exportProgressPercent = null;
    const skipRemux =
      $editor.previewStrategy === 'Preview remux' ||
      !$editor.metadata ||
      planPreview({
        codec: $editor.metadata.codec,
        pixelFormat: $editor.metadata.pixelFormat,
        container: $editor.metadata.container,
        fileSize: $editor.metadata.fileSize,
      }) !== 'remux';
    openingMessage = skipRemux
      ? 'This file can’t play here directly, so Cutdown is making a quick preview copy. Your export still uses the original.'
      : 'Repackaging the file so it can play here (no quality change)…';
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message: skipRemux ? 'The preview player can’t decode this file. Making a preview copy…' : 'The preview player had trouble. Repackaging the file for preview…',
      },
    }));

    try {
      const previousPreview = $editor.previewTempPath;
      const result = await invoke<PreviewResult>('prepare_preview', {
        params: {
          inputPath: $editor.currentFile,
          forceProxy: skipRemux,
        },
      });

      await cleanupPreview(previousPreview);
      editor.update((state) => ({
        ...state,
        videoSrc: mediaSrcFromPath(result.previewPath),
        previewTempPath: result.previewPath,
        previewStrategy: result.strategy,
        currentTime: 0,
        exportStatus: {
          state: 'idle',
          message: `Preview copy ready; export still uses the original. ${READY_HINT}`,
        },
      }));
      preview?.seekTo(0);
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: detail,
        },
      }));
      pushToast(detail, 'error');
    } finally {
      previewFallbackRunning = false;
    }
  }

  async function revealExport(): Promise<void> {
    const path = $editor.exportStatus.outputPath;

    if (!path) {
      return;
    }

    try {
      await invoke('reveal_in_explorer', { path });
    } catch (error) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          ...state.exportStatus,
          state: 'error',
          message: error instanceof Error ? error.message : String(error),
        },
      }));
    }
  }
</script>

<svelte:window
  on:keydown={handleKeydown}
  on:pointermove={updateWorkspaceResize}
  on:pointerup={stopWorkspaceResize}
  on:pointercancel={stopWorkspaceResize}
/>

<ToastHost />

<main class="shell" class:shell--dragover={dragOver}>
  {#if !ffmpegAvailable}
    <div class="shell__alerts">
      {#if !ffmpegAvailable}
        <div class="ffmpeg-banner" role="alert">
          <span>
            {ffmpegInstalling
              ? ffmpegInstallMessage || 'Downloading ffmpeg…'
              : ffmpegStatus ||
                (isWindows
                  ? 'ffmpeg is required for export and clip analysis. Re-run the installer (downloads latest ffmpeg) or use the button below (~80 MB).'
                  : 'ffmpeg is required for export and clip analysis. Install it with your package manager (sudo apt install ffmpeg), then restart Cutdown.')}
            {#if ffmpegInstalling && ffmpegInstallPercent !== null}
              {' '}({Math.round(ffmpegInstallPercent)}%)
            {/if}
          </span>
          {#if isWindows}
            <button
              type="button"
              class="secondary"
              disabled={ffmpegInstalling}
              on:click={() => void installFfmpeg()}
            >
              {ffmpegInstalling ? 'Downloading…' : 'Install ffmpeg'}
            </button>
          {/if}
          <button type="button" class="secondary" title="Open Settings" on:click={() => void openSettings()}>Settings</button>
        </div>
      {/if}
    </div>
  {/if}

  <EditorToolbar
    filePath={$editor.currentFile}
    {metadata}
    metadataDetail={clipMetadataDetail}
    {recentSources}
    canUndo={segmentHistory.length > 0}
    canRedo={redoHistory.length > 0}
    {canExport}
    busy={openingClip}
    on:open={() => void chooseClip()}
    on:openProject={() => void openProject()}
    on:latestReplay={() => void loadLatestReplay()}
    on:save={() => void saveProject()}
    on:openRecent={(event) => void openIncomingPath(event.detail.path)}
    on:undo={undoSegmentEdit}
    on:redo={redoSegmentEdit}
    on:help={() => (shortcutsModalOpen = true)}
    on:history={() => (historyDrawerOpen = true)}
    on:settings={() => void openSettings()}
    {exportPanelOpen}
    on:export={toggleExportPanel}
  />

  {#if exportPanelOpen}
    <div class="export-panel-anchor">
    <ExportPanel
      presetIds={quickPresetIds}
      presetId={QUICK_PRESET_IDS.includes(exportPresetId) ? exportPresetId : 'lossless-trim'}
      exportSeconds={quickExportSeconds}
      partCount={quickRanges.length}
      trimmedToRange={rangeIsNarrowed}
      sourceSeconds={duration}
      sourceBytes={metadata?.fileSize ?? 0}
      {reencodeReasons}
      destinationName={quickDestination.split(/[\\/]/).pop() ?? ''}
      destinationPath={quickDestination}
      on:choose={(event) => (exportPresetId = event.detail.presetId)}
      on:export={() => void quickExport()}
      on:moreOptions={openFullExportOptions}
      on:close={() => (exportPanelOpen = false)}
    />
    </div>
  {/if}

  <section class="editor-workspace" bind:this={workspaceEl}>
  <section class="preview-panel" style:flex={`${workspaceSplitRatio} 1 0`}>
    {#if openingClip || previewFallbackRunning}
      <div class="opening-overlay" aria-busy="true">
        <span>
          {openingClip ? openingMessage : $editor.exportStatus.message || 'Building a playable preview…'}
          {#if exportProgressPercent !== null}
            {' '}({Math.round(exportProgressPercent)}%)
          {/if}
        </span>
      </div>
    {/if}
    <PreviewToolbar
      hasClip={Boolean($editor.currentFile)}
      proxyRunning={previewFallbackRunning}
      bind:cropEnabled
      bind:cropLockAspect
      {cropAspect}
      bind:cropCustomAspectW
      bind:cropCustomAspectH
      bind:playbackRate={previewPlaybackRate}
      on:aspect={(event) => applyAspectCrop(event.detail.aspect)}
      on:fit={() => preview?.fitToView()}
      on:proxy={() => void prepareProxyPreview()}
    />
    <VideoPreview
      bind:this={preview}
      playbackRate={previewPlaybackRate}
      src={$editor.videoSrc}
      currentTime={$editor.currentTime}
      loopEnabled={rangeLoopPlayback}
      loopStart={normalizedRange?.start ?? null}
      loopEnd={normalizedRange?.end ?? null}
      volume={clipVolume}
      {cropEnabled}
      bind:cropRect
      lockAspectRatio={cropLockAspect}
      lockedAspectRatio={cropLockedAspectRatio}
      on:cropChange={(event) => {
        cropRect = event.detail.rect;
      }}
      on:metadata={() => {}}
      on:previewready={handlePreviewReady}
      on:error={(event) => void handlePreviewError(event.detail.message)}
      on:open={() => void chooseClip()}
      on:playstate={(event) => handlePreviewPlayState(event.detail.playing)}
      on:ended={handlePreviewEnded}
      on:timeupdate={(event) => handlePreviewTimeUpdate(event.detail.currentTime)}
    />
  </section>

  <button
    type="button"
    class="workspace-splitter"
    bind:this={workspaceSplitter}
    aria-label="Resize preview and timeline"
    on:pointerdown={startWorkspaceResize}
  ></button>

  <div class="timeline-pane" style:flex={`${1 - workspaceSplitRatio} 1 0`}>
    <TransportRow
      hasClip={Boolean($editor.currentFile)}
      canEdit={canExport}
      canDeleteSegment={Boolean(selectedSegment) && segments.length > 1}
      playing={previewPlaying}
      on:split={splitAtCurrentTime}
      on:deleteSegment={deleteSelectedSegment}
      on:markIn={() => setRangeMarker('start', $editor.currentTime)}
      on:markOut={() => setRangeMarker('end', $editor.currentTime)}
      on:addMarker={() => addBookmarkMarker($editor.currentTime)}
      on:togglePlay={() => preview?.togglePlayback()}
    />
  <Timeline
    bind:this={timeline}
    disabled={!$editor.currentFile}
    {duration}
    currentTime={$editor.currentTime}
    {segments}
    {bookmarks}
    {selectedBookmarkId}
    waveformPeaks={waveformPeaks}
    waveformLoading={waveformLoading}
    {thumbnails}
    videoAspect={metadata && metadata.height > 0 ? metadata.width / metadata.height : 16 / 9}
    sourceDuration={metadata?.duration ?? 0}
    selectedSegmentId={$editor.selectedSegmentId}
    rangeStart={normalizedRange?.start ?? null}
    rangeEnd={normalizedRange?.end ?? null}
    zoom={timelineZoom}
    bind:videoTrackHeight
    bind:audioTrackHeight
    audioCodec={metadata?.audioCodec ?? null}
    audioChannels={metadata?.audioChannels ?? null}
    on:seek={(event) => seekTo(event.detail.seconds)}
    on:selectSegment={(event) => selectSegment(event.detail.id)}
    on:rangeChange={(event) => {
      rangeStart = event.detail.start;
      rangeEnd = event.detail.end;
      loopNarrowedRange();
    }}
    on:zoomChange={(event) => (timelineZoom = event.detail.zoom)}
    on:trackHeightChange={updateTrackHeights}
    on:splitAt={(event) => splitAtTime(event.detail.seconds)}
    on:deleteSelected={deleteSelectedSegment}
    on:zoomFit={zoomToFit}
    on:zoomRange={zoomToRange}
    on:clearRange={clearRange}
    on:splitRange={splitAtRangeMarkers}
    on:keepRange={keepOnlyRange}
    on:trimOutsideRange={deleteOutsideRange}
    on:toggleRangeLoop={toggleRangeLoop}
    on:snapToRangeStart={snapToRangeStart}
    on:snapToRangeEnd={snapToRangeEnd}
    on:previousMarker={goToPreviousMarker}
    on:nextMarker={goToNextMarker}
    on:deleteSelectedMarker={deleteSelectedBookmark}
    on:reorderSegment={(event) => reorderSegment(event.detail.id, event.detail.toIndex)}
    on:segmentResize={(event) =>
      resizeSegment(event.detail.id, event.detail.sourceStart, event.detail.sourceEnd, {
        recordUndo: event.detail.recordUndo,
        commit: event.detail.commit,
      })}
    on:bookmarkClick={(event) => {
      const bookmark = bookmarks.find((entry) => entry.id === event.detail.id);
      if (bookmark) {
        seekTo(bookmark.time);
      }
    }}
    on:bookmarkSelect={(event) => {
      selectedBookmarkId = event.detail.id;
    }}
    on:bookmarkEdit={(event) => openBookmarkLabelEditor(event.detail.id)}
    on:bookmarkRemove={(event) => removeBookmark(event.detail.id)}
  />
  </div>
  </section>

  <StatusBar
    {outputDuration}
    segmentCount={segments.length}
    selectedSegment={selectedSegment ?? null}
    range={normalizedRange ?? null}
    rangeLoop={rangeLoopPlayback}
    {canUseRange}
    volume={clipVolume}
    volumeEnabled={Boolean($editor.currentFile && metadata?.audioCodec)}
    exportStatus={$editor.exportStatus}
    statusLabel={exportEta ? `${exportStatusLabel} · ${exportEta}` : exportStatusLabel}
    note={editNote}
    progressPercent={exportProgressPercent}
    dismissible={statusDismissible}
    shareUrl={lastShareUrl}
    on:toggleLoop={toggleRangeLoop}
    on:volume={(event) => (clipVolume = clamp(event.detail.volume, 0, 1))}
    on:dismiss={dismissExportStatus}
    on:copyFile={() => void copyExportedFile()}
    on:copyPath={() => void invoke('copy_text_to_clipboard', { text: $editor.exportStatus.outputPath ?? '' })}
    on:copyShareLink={() => void copyShareLink(lastShareUrl ?? '')}
    on:reveal={revealExport}
    on:upload={() => openUploadPicker($editor.exportStatus.outputPath ?? '')}
    on:openExport={openExportModal}
    on:cancel={() => void cancelExport()}
  />
</main>

<ExportModal
  open={exportModalOpen}
  {outputDirectory}
  bind:outputFileName
  bind:exportType
  bind:audioPresetId
  segmentCount={segments.length}
  duration={outputDuration}
  {rangeDuration}
  canExportRange={canUseRange}
  {exportMode}
  presets={exportPresets}
  presetId={exportPresetId}
  bind:accurateTrim={accurateTrimExport}
  bind:stripAudio={stripAudioExport}
  bind:batchPerSegment={exportBatchPerSegment}
  bind:queueUploadAfterExport={queueUploadAfterExport}
  bind:fadeInSeconds={fadeInSeconds}
  bind:fadeOutSeconds={fadeOutSeconds}
  {usesStreamCopy}
  {streamCopyBlockers}
  hasAudio={Boolean(metadata?.audioCodec)}
  exportBusy={exportQueueProcessing}
  {uploadTargetsConfigured}
  on:close={() => (exportModalOpen = false)}
  on:chooseOutput={chooseOutput}
  on:exportModeChange={(event) => (exportMode = event.detail.mode)}
  on:presetChange={(event) => (exportPresetId = event.detail.presetId)}
  on:openUploadSettings={() => {
    exportModalOpen = false;
    void openSettings('upload');
  }}
  on:confirm={exportClip}
/>

<UploadTargetModal
  open={uploadTargetModalOpen}
  filePath={uploadTargetPath}
  providers={uploadPickerProviders}
  bind:selectedProviderId={selectedUploadProviderId}
  busy={historyBusyPath === uploadTargetPath}
  on:close={() => (uploadTargetModalOpen = false)}
  on:openSettings={() => {
    uploadTargetModalOpen = false;
    void openSettings('upload');
  }}
  on:upload={(event) => void uploadClip(uploadTargetPath, event.detail.providerId)}
/>

<ConfirmModal
  open={watchFolderConfirmOpen}
  title="Open new clip"
  message="Replace the current clip? Unsaved segment edits will be lost."
  confirmLabel="Replace clip"
  on:close={() => {
    watchFolderConfirmOpen = false;
    watchFolderPendingPath = '';
  }}
  on:confirm={confirmWatchFolderReplace}
/>

<ConfirmModal
  open={clearHistoryConfirmOpen}
  title="Clear clip history"
  message="Remove all entries from clip history? This does not delete exported files on disk."
  confirmLabel="Clear history"
  danger={true}
  on:close={() => (clearHistoryConfirmOpen = false)}
  on:confirm={() => void clearHistory()}
/>

<RelinkSourceModal
  open={relinkModalOpen}
  missingPath={relinkMissingPath}
  projectName={relinkProjectName}
  on:close={() => {
    relinkModalOpen = false;
    pendingProject = null;
  }}
  on:choose={() => void relinkProjectSource()}
/>

<SettingsModal
  visible={settingsModalOpen}
  initialTab={settingsInitialTab}
  {watchFolder}
  {watchFolderEnabled}
  {defaultExportDir}
  {exportPresetId}
  {preferGpuEncoding}
  {runAtStartup}
  {startMinimizedToTray}
  {closeToTray}
  {appVersion}
  {uploadProviders}
  {defaultUploadProviderId}
  {customExportPresets}
  {ffmpegStatus}
  {gpuEncoders}
  on:close={() => void closeSettingsModal()}
  on:restoreTrayHint={restoreTrayHint}
  on:checkForUpdates={() => void checkForUpdates({ manual: true })}
  on:error={(event) => {
    pushToast(event.detail.message, 'error');
  }}
  on:saved={async (event) => {
    watchFolder = event.detail.watchFolder;
    watchFolderEnabled = event.detail.watchFolderEnabled;
    defaultExportDir = event.detail.defaultExportDir;
    exportPresetId = event.detail.lastPresetId;
    preferGpuEncoding = event.detail.preferGpuEncoding;
    runAtStartup = event.detail.runAtStartup;
    startMinimizedToTray = event.detail.startMinimizedToTray;
    closeToTray = event.detail.closeToTray;
    uploadProviders = event.detail.uploadProviders;
    defaultUploadProviderId = event.detail.defaultUploadProviderId;
    customExportPresets = event.detail.customExportPresets;
    syncUploadProviderSelection();
    try {
      await reloadUploadProvidersFromDisk();
    } catch (error) {
      pushToast(error instanceof Error ? error.message : String(error), 'error');
    }
    void refreshExportPresets();
  }}
  on:windowsSaved={(event) => {
    runAtStartup = event.detail.runAtStartup;
    startMinimizedToTray = event.detail.startMinimizedToTray;
    closeToTray = event.detail.closeToTray;
  }}
/>

<MarkerLabelModal
  open={markerLabelModalOpen}
  initialLabel={bookmarks.find((bookmark) => bookmark.id === editingBookmarkId)?.label ?? ''}
  on:close={() => {
    markerLabelModalOpen = false;
    editingBookmarkId = null;
  }}
  on:save={(event) => {
    if (editingBookmarkId) {
      updateBookmarkLabel(editingBookmarkId, event.detail.label);
    }
  }}
/>

<ShortcutsModal open={shortcutsModalOpen} on:close={() => (shortcutsModalOpen = false)} />

<UpdateModal
  visible={updateModalOpen}
  version={updateInstallVersion}
  notes={updateInstallNotes}
  currentVersion={appVersion}
  installing={updateInstalling}
  installError={updateInstallError}
  downloadedBytes={updateDownloadedBytes}
  totalBytes={updateTotalBytes}
  on:close={() => {
    if (!updateInstalling) {
      updateModalOpen = false;
    }
  }}
  on:install={() => void installPendingUpdate()}
/>

<ClipHistoryDrawer
  open={historyDrawerOpen}
  entries={clipHistory}
  busyPath={historyBusyPath}
  uploadConfigured={uploadTargetsConfigured}
  on:close={() => (historyDrawerOpen = false)}
  on:reveal={(event) => void invoke('reveal_in_explorer', { path: event.detail.path })}
  on:openClip={(event) => void openIncomingPath(event.detail.path)}
  on:copyPath={(event) => void invoke('copy_text_to_clipboard', { text: event.detail.path })}
  on:copyLink={(event) => void copyShareLink(event.detail.url)}
  on:upload={(event) => openUploadPicker(event.detail.path)}
  on:openSettings={() => {
    historyDrawerOpen = false;
    void openSettings();
  }}
  on:remove={(event) => void removeHistoryEntry(event.detail.path)}
  on:clear={requestClearHistory}
/>
