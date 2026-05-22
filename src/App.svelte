<script lang="ts">
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { confirm, open, save } from '@tauri-apps/plugin-dialog';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
  import { onMount, tick } from 'svelte';
  import { get } from 'svelte/store';
  import ClipHistoryDrawer from './components/ClipHistoryDrawer.svelte';
  import ConfirmModal from './components/ConfirmModal.svelte';
  import ExportActivity from './components/ExportActivity.svelte';
  import ExportModal from './components/ExportModal.svelte';
  import IconButton from './components/IconButton.svelte';
  import ProgressBar from './components/ProgressBar.svelte';
  import RelinkSourceModal from './components/RelinkSourceModal.svelte';
  import SettingsModal from './components/SettingsModal.svelte';
  import MarkerLabelModal from './components/MarkerLabelModal.svelte';
  import ShortcutsModal from './components/ShortcutsModal.svelte';
  import Timeline from './components/Timeline.svelte';
  import ToastHost from './components/ToastHost.svelte';
  import UploadTargetModal from './components/UploadTargetModal.svelte';
  import VideoPreview from './components/VideoPreview.svelte';
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
  import { buildPerSegmentJobs, type ExportJob } from './lib/exportQueue';
  import { createSequencePlaybackDriver } from './lib/sequencePlayback';
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
    createFullSegment,
    editor,
    totalSegmentDuration,
    type EditorState,
    type TimelineSegment,
    type VideoMetadata,
  } from './stores/editor';
  import { pushToast } from './stores/toasts';

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

  type SourceSession = {
    sourcePath: string;
    segments: TimelineSegment[];
    selectedSegmentId: string | null;
    rangeStart: number | null;
    rangeEnd: number | null;
    cropEnabled: boolean;
    cropRect: NormalizedCropRect;
    clipVolume: number;
    currentTime: number | null;
    bookmarks?: TimelineBookmark[];
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
  let persistSessionTimer: ReturnType<typeof setTimeout> | null = null;
  let clipHistory: ClipHistoryEntry[] = [];
  let historyBusyPath: string | null = null;
  let cropEnabled = false;
  let cropAspect: 'free' | '16:9' | '9:16' | '4:3' | '1:1' | 'custom' = 'free';
  let cropLockAspect = false;
  let cropCustomAspectW = 16;
  let cropCustomAspectH = 9;
  let cropRect: NormalizedCropRect = { x: 0.05, y: 0.05, width: 0.9, height: 0.9 };
  let recentMenuOpen = false;
  let uploadProviders: UploadProvider[] = [];
  let uploadProviderSummaries: UploadProviderSummary[] = [];
  let uploadPickerProviders: UploadProviderSummary[] = [];
  let defaultUploadProviderId: string | null = null;
  let selectedUploadProviderId: string | null = null;
  let exportPresets: PresetInfo[] = [];
  let customExportPresets: CustomExportPreset[] = [];
  let exportPresetId = 'lossless-trim';
  let gpuEncoders: string[] = [];
  let preferGpuEncoding = true;
  let defaultExportDir: string | null = null;
  let runAtStartup = false;
  let exportMode: 'sequence' | 'range' = 'sequence';
  let rangeLoopPlayback = false;
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
  let timelineZoom = 28;
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
  let waveformLoading = false;
  let waveformLoadGeneration = 0;
  let pendingWaveform: { path: string; hasAudio: boolean; generation: number; duration: number } | null =
    null;
  let waveformDeferTimer: ReturnType<typeof setTimeout> | null = null;
  let markerLabelModalOpen = false;
  let editingBookmarkId: string | null = null;
  let currentWindowTitle = '';
  let openingClip = false;
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
  let pendingProject: {
    sourcePath: string;
    segments: TimelineSegment[];
    selectedSegmentId: string | null;
    rangeStart: number | null;
    rangeEnd: number | null;
    cropEnabled: boolean;
    cropRect: NormalizedCropRect;
    clipVolume: number;
    currentTime: number | null;
    bookmarks: TimelineBookmark[];
    exportPresetId: string | null;
    accurateTrim: boolean;
    stripAudio: boolean;
  } | null = null;
  let trayHintDismissed = false;

  const TRAY_HINT_KEY = 'cutdown-tray-hint-dismissed';
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
    void bootstrapApp();

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

    const videoExtensions = new Set(['mp4', 'mkv', 'mov', 'avi', 'webm', 'ts', 'flv']);
    const appWindow = getCurrentWindow();
    const unlistenDragDrop = appWindow.onDragDropEvent((event) => {
      if (event.payload.type === 'over') {
        dragOver = true;
      } else if (event.payload.type === 'drop') {
        dragOver = false;
        const path = event.payload.paths.find((candidate) => {
          const ext = candidate.split(/[\\/]/).pop()?.split('.').pop()?.toLowerCase() ?? '';
          return videoExtensions.has(ext);
        });
        if (path) {
          void openClipPath(path);
        }
      } else {
        dragOver = false;
      }
    });

    return () => {
      sequencePlayback.dispose();
      void unlistenFfmpegInstall.then((stop) => stop());
      void unlistenExport.then((stop) => stop());
      void unlistenWatch.then((stop) => stop());
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

  async function bootstrapApp(): Promise<void> {
    try {
      const [settings, ffmpeg, presets, encoders, launchPath] = await Promise.all([
        invoke<AppSettings>('get_settings'),
        invoke<FfmpegCheckResult>('check_ffmpeg'),
        invoke<PresetInfo[]>('list_presets'),
        invoke<string[]>('detect_gpu_encoders'),
        invoke<string | null>('get_launch_path'),
      ]);

      watchFolder = settings.watchFolder;
      watchFolderEnabled = settings.watchFolderEnabled;
      defaultExportDir = settings.defaultExportDir ?? settings.lastExportDir;
      exportPresetId = settings.lastPresetId || 'lossless-trim';
      preferGpuEncoding = settings.preferGpuEncoding;
      runAtStartup = settings.runAtStartup;
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

      if (launchPath) {
        await openClipPath(launchPath);
      }
    } catch (error) {
      ffmpegStatus = error instanceof Error ? error.message : String(error);
    }

    if (!(await isPermissionGranted())) {
      await requestPermission();
    }
  }

  function dismissTrayHint(): void {
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
    pushToast('Tray minimize tip restored.', 'info');
  }

  function toggleRecentMenu(): void {
    recentMenuOpen = !recentMenuOpen;
  }

  function handleWindowClick(event: MouseEvent): void {
    if (!recentMenuOpen) {
      return;
    }

    const target = event.target as HTMLElement;
    if (!target.closest('.toolbar-recent')) {
      recentMenuOpen = false;
    }
  }

  function openRecentSource(path: string): void {
    recentMenuOpen = false;
    void openClipPath(path);
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
    if (hasUnsavedEdits()) {
      watchFolderPendingPath = path;
      watchFolderConfirmOpen = true;
      return;
    }

    await openClipPath(path);
  }

  function confirmWatchFolderReplace(): void {
    watchFolderConfirmOpen = false;
    const path = watchFolderPendingPath;
    watchFolderPendingPath = '';
    if (path) {
      void openClipPath(path);
    }
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
  $: exportPresetDisplayName = exportPresets.find((preset) => preset.id === exportPresetId)?.name ?? exportPresetId;
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
  $: streamCopyBlockers = (() => {
    if (!selectedExportPreset?.lossless) {
      return [];
    }

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
  $: outputPath = joinOutputPath(outputDirectory, outputFileName);
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

  function schedulePersistSession(): void {
    if (!$editor.currentFile || !metadata) {
      return;
    }

    if (persistSessionTimer) {
      clearTimeout(persistSessionTimer);
    }

    persistSessionTimer = setTimeout(() => {
      persistSessionTimer = null;
      void persistSession();
    }, 500);
  }

  async function persistSession(): Promise<void> {
    const file = get(editor).currentFile;
    if (!file || !metadata) {
      return;
    }

    try {
      await invoke('save_source_session', {
        session: {
          sourcePath: file,
          segments: get(editor).segments,
          selectedSegmentId: get(editor).selectedSegmentId,
          rangeStart,
          rangeEnd,
          cropEnabled,
          cropRect,
          clipVolume,
          currentTime: get(editor).currentTime,
          bookmarks,
        },
        duration: metadata.duration,
      });
    } catch {
      // Session persistence is best-effort.
    }
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
    pushToast('Undid last edit.');
    schedulePersistSession();
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
    pushToast('Redid edit.');
    schedulePersistSession();
  }

  async function chooseClip(): Promise<void> {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Video',
          extensions: ['mp4', 'mkv', 'mov', 'avi', 'webm', 'ts', 'flv'],
        },
      ],
    });

    if (typeof selected !== 'string') {
      return;
    }

    await openClipPath(selected);
  }

  async function openClipPath(selected: string): Promise<void> {
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
    if (persistSessionTimer) {
      clearTimeout(persistSessionTimer);
      persistSessionTimer = null;
    }

    segmentHistory = [];
    redoHistory = [];
    rangeStart = null;
    rangeEnd = null;
    bookmarks = [];
    selectedBookmarkId = null;
    waveformPeaks = [];
    cancelPendingWaveform();
    waveformLoadGeneration += 1;
    rangeLoopPlayback = false;
    sequencePlayback.reset();
    clipVolume = 1;
    cropEnabled = false;
    cropRect = { x: 0.05, y: 0.05, width: 0.9, height: 0.9 };
    exportMode = 'sequence';
    openingClip = true;
    statusDismissed = false;
    editor.update((state) => ({
      ...state,
      currentFile: selected,
      videoSrc: convertFileSrc(selected),
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
      const saved = await invoke<SourceSession | null>('get_source_session', {
        path: selected,
        duration: probed.duration,
      });

      if (saved?.segments?.length) {
        rangeStart = saved.rangeStart ?? 0;
        rangeEnd = saved.rangeEnd ?? probed.duration;
        clipVolume = clamp(saved.clipVolume ?? 1, 0, 1);
        cropEnabled = saved.cropEnabled;
        cropRect = saved.cropRect;
        bookmarks = (saved.bookmarks ?? []).map((bookmark) => ({
          id: bookmark.id,
          time: bookmark.time,
          label: bookmark.label,
        }));
        editor.update((state) => ({
          ...state,
          metadata: probed,
          segments: saved.segments,
          selectedSegmentId: saved.selectedSegmentId,
          currentTime: clamp(saved.currentTime ?? 0, 0, probed.duration),
          exportStatus: {
            state: 'idle',
            message: `Restored session for ${formatBytes(probed.fileSize)} ${probed.codec.toUpperCase()} clip.`,
          },
        }));
        await tick();
        seekTo(get(editor).currentTime);
      } else {
        rangeStart = 0;
        rangeEnd = probed.duration;
        editor.update((state) => ({
          ...state,
          metadata: probed,
          segments: [createFullSegment(probed.duration)],
          selectedSegmentId: null,
          exportStatus: {
            state: 'idle',
            message: `Loaded ${formatBytes(probed.fileSize)} ${probed.codec.toUpperCase()} clip with native preview.`,
          },
        }));
      }

      queueWaveformAfterPreview(selected, probed.audioCodec != null, probed.duration);
      syncExportDefaultsForClip();
      if (needsProxyPreview(probed)) {
        editor.update((state) => ({
          ...state,
          exportStatus: {
            state: 'idle',
            message: `${state.exportStatus.message} Heavy codec or large file — use Proxy preview if playback stutters.`,
          },
        }));
      }
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

  function splitAtTime(splitTime: number, options: { recordUndo?: boolean; persist?: boolean; toast?: boolean } = {}): void {
    const { recordUndo = true, persist = true, toast = true } = options;
    if (!canExport) {
      return;
    }

    const targetSegment = $editor.segments.find(
      (segment) => splitTime > segment.sourceStart + 0.05 && splitTime < segment.sourceEnd - 0.05,
    );

    if (!targetSegment) {
      pushToast('Move the playhead inside a segment to split.', 'info');
      return;
    }

    if (recordUndo) {
      pushUndoSnapshot();
    }
    editor.update((state) => ({
      ...state,
      segments: state.segments.flatMap((segment) => {
        const insideSegment = splitTime > segment.sourceStart + 0.05 && splitTime < segment.sourceEnd - 0.05;

        if (!insideSegment) {
          return [segment];
        }

        return [
          {
            ...segment,
            sourceEnd: splitTime,
          },
          {
            id: crypto.randomUUID(),
            sourceStart: splitTime,
            sourceEnd: segment.sourceEnd,
          },
        ];
      }),
      selectedSegmentId: null,
    }));
    if (toast) {
      pushToast(`Split at ${formatTime(splitTime)}.`);
    }
    refreshPreviewPlaybackAfterSegmentEdit();
    if (persist) {
      schedulePersistSession();
    }
  }

  function splitAtCurrentTime(): void {
    splitAtTime($editor.currentTime);
  }

  function deleteSelectedSegment(): void {
    if (!$editor.selectedSegmentId) {
      return;
    }

    if ($editor.segments.length <= 1) {
      pushToast('At least one segment is required.', 'info');
      return;
    }

    pushUndoSnapshot();
    editor.update((state) => ({
      ...state,
      segments: state.segments.filter((segment) => segment.id !== state.selectedSegmentId),
      selectedSegmentId: null,
    }));
    pushToast('Deleted selected segment.');
    refreshPreviewPlaybackAfterSegmentEdit();
    schedulePersistSession();
  }

  function duplicateSelectedSegment(): void {
    if (!$editor.selectedSegmentId) {
      return;
    }

    const selected = $editor.segments.find((segment) => segment.id === $editor.selectedSegmentId);
    if (!selected) {
      return;
    }

    const duplicate = {
      id: crypto.randomUUID(),
      sourceStart: selected.sourceStart,
      sourceEnd: selected.sourceEnd,
    };

    pushUndoSnapshot();
    editor.update((state) => {
      const index = state.segments.findIndex((segment) => segment.id === selected.id);
      const nextSegments = [...state.segments];
      nextSegments.splice(index + 1, 0, duplicate);

      return {
        ...state,
        segments: nextSegments,
        selectedSegmentId: duplicate.id,
      };
    });
    pushToast('Duplicated selected segment.');
    refreshPreviewPlaybackAfterSegmentEdit();
    schedulePersistSession();
  }

  function reorderSegment(id: string, toIndex: number): void {
    pushUndoSnapshot();
    editor.update((state) => {
      const fromIndex = state.segments.findIndex((segment) => segment.id === id);
      if (fromIndex < 0) {
        return state;
      }

      const nextSegments = [...state.segments];
      const [moved] = nextSegments.splice(fromIndex, 1);
      const targetIndex = clamp(toIndex, 0, nextSegments.length);
      nextSegments.splice(targetIndex, 0, moved);

      return {
        ...state,
        segments: nextSegments,
        selectedSegmentId: id,
        exportStatus: {
          state: 'idle',
          message: 'Reordered segment.',
        },
      };
    });
    refreshPreviewPlaybackAfterSegmentEdit();
    schedulePersistSession();
  }

  function splitAtRangeMarkers(): void {
    if (!canUseRange || !normalizedRange) {
      return;
    }

    pushUndoSnapshot();
    splitAtTime(normalizedRange.start, { recordUndo: false, persist: false, toast: false });
    splitAtTime(normalizedRange.end, { recordUndo: false, persist: false, toast: false });
    pushToast(`Split at I/O markers.`);
    refreshPreviewPlaybackAfterSegmentEdit();
    schedulePersistSession();
  }

  function keepOnlyRange(): void {
    if (!canUseRange || !normalizedRange) {
      return;
    }

    pushUndoSnapshot();
    editor.update((state) => ({
      ...state,
      segments: [
        {
          id: crypto.randomUUID(),
          sourceStart: normalizedRange.start,
          sourceEnd: normalizedRange.end,
        },
      ],
      selectedSegmentId: null,
      exportStatus: {
        state: 'idle',
        message: `Kept range ${formatTime(normalizedRange.start)} - ${formatTime(normalizedRange.end)}.`,
      },
    }));
    pushToast(`Kept only ${formatTime(normalizedRange.start)} – ${formatTime(normalizedRange.end)}.`, 'success');
    seekTo(normalizedRange.start);
    schedulePersistSession();
  }

  function deleteOutsideRange(): void {
    if (!canUseRange || !normalizedRange) {
      return;
    }

    const { start, end } = normalizedRange;
    pushUndoSnapshot();
    editor.update((state) => {
      const trimmedSegments = state.segments.flatMap((segment) => {
        const overlapStart = Math.max(segment.sourceStart, start);
        const overlapEnd = Math.min(segment.sourceEnd, end);

        if (overlapEnd <= overlapStart + 0.05) {
          return [];
        }

        return [
          {
            ...segment,
            sourceStart: overlapStart,
            sourceEnd: overlapEnd,
          },
        ];
      });

      return {
        ...state,
        segments:
          trimmedSegments.length > 0
            ? trimmedSegments
            : [
                {
                  id: crypto.randomUUID(),
                  sourceStart: start,
                  sourceEnd: end,
                },
              ],
        selectedSegmentId: null,
        exportStatus: {
          state: 'idle',
          message: 'Removed footage outside the I/O range.',
        },
      };
    });
    refreshPreviewPlaybackAfterSegmentEdit();
    schedulePersistSession();
  }

  function handleVolumeInput(event: Event): void {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    clipVolume = clamp(value / 100, 0, 1);
    schedulePersistSession();
  }

  function toggleRangeLoop(): void {
    if (!canUseRange) {
      return;
    }

    rangeLoopPlayback = !rangeLoopPlayback;

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
    const nextTime = clamp(seconds, 0, duration);

    if (marker === 'start') {
      rangeStart = nextTime;
      rangeEnd = rangeEnd ?? duration;
    } else {
      rangeStart = rangeStart ?? 0;
      rangeEnd = nextTime;
    }

    schedulePersistSession();
  }

  function clearRange(): void {
    rangeStart = null;
    rangeEnd = null;
    schedulePersistSession();
  }

  function defaultBookmarkLabel(): string {
    return `Marker ${bookmarks.length + 1}`;
  }

  function addBookmarkMarker(seconds: number, label?: string): void {
    if (!metadata) {
      return;
    }

    const time = clamp(seconds, 0, metadata.duration);
    if (bookmarks.some((bookmark) => Math.abs(bookmark.time - time) < BOOKMARK_DEDUPE_SECONDS)) {
      pushToast('A marker already exists at this time.', 'info');
      return;
    }

    const entry: TimelineBookmark = {
      id: crypto.randomUUID(),
      time,
      label: label?.trim() || defaultBookmarkLabel(),
    };
    bookmarks = [...bookmarks, entry].sort((left, right) => left.time - right.time);
    selectedBookmarkId = entry.id;
    schedulePersistSession();
  }

  function updateBookmarkLabel(id: string, label: string): void {
    const trimmed = label.trim();
    bookmarks = bookmarks.map((bookmark) =>
      bookmark.id === id ? { ...bookmark, label: trimmed || formatTime(bookmark.time) } : bookmark,
    );
    schedulePersistSession();
  }

  function removeBookmark(id: string): void {
    bookmarks = bookmarks.filter((bookmark) => bookmark.id !== id);
    if (selectedBookmarkId === id) {
      selectedBookmarkId = null;
    }
    schedulePersistSession();
  }

  function deleteSelectedBookmark(): void {
    if (!selectedBookmarkId) {
      return;
    }

    removeBookmark(selectedBookmarkId);
    pushToast('Marker deleted.');
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
    uploadProviders = resolveUploadProvidersFromSettings(settings);
    defaultUploadProviderId =
      readDefaultUploadProviderId(settingsRecord) ??
      uploadProviders.find((provider) => provider.enabled)?.id ??
      null;
    customExportPresets = parseCustomPresetsFromSettings(settings.customExportPresets);
    syncUploadProviderSelection();
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
      schedulePersistSession();
      return;
    }

    cropLockAspect = true;

    if (!metadata) {
      schedulePersistSession();
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
    schedulePersistSession();
  }

  async function refreshClipHistory(): Promise<void> {
    clipHistory = await invoke<ClipHistoryEntry[]>('list_clip_history');
  }

  async function removeHistoryEntry(outputPath: string): Promise<void> {
    clipHistory = await invoke<ClipHistoryEntry[]>('remove_clip_history_entry', { outputPath });
    pushToast('Removed from clip history.');
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
    pushToast('Clip history cleared.');
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
    const isTextInput =
      target?.tagName === 'TEXTAREA' ||
      target?.isContentEditable ||
      (target instanceof HTMLInputElement &&
        !['button', 'checkbox', 'radio', 'range'].includes(target.type));

    if (event.key === 'Escape') {
      if (
        exportModalOpen ||
        settingsModalOpen ||
        historyDrawerOpen ||
        shortcutsModalOpen ||
        markerLabelModalOpen
      ) {
        event.preventDefault();
        closeModals();
        return;
      }

      if ($editor.selectedSegmentId) {
        event.preventDefault();
        deselectSegment();
        return;
      }
    }

    if (event.key === '?' && !event.ctrlKey && !event.altKey) {
      event.preventDefault();
      shortcutsModalOpen = true;
      return;
    }

    if (isTextInput) {
      return;
    }

    if (event.ctrlKey && event.key.toLowerCase() === 'z') {
      event.preventDefault();
      if (event.shiftKey) {
        redoSegmentEdit();
      } else {
        undoSegmentEdit();
      }
      return;
    }

    if (event.ctrlKey && event.key.toLowerCase() === 'y') {
      event.preventDefault();
      redoSegmentEdit();
      return;
    }

    if (event.ctrlKey && event.key.toLowerCase() === 'd') {
      event.preventDefault();
      duplicateSelectedSegment();
      return;
    }

    if (!canExport) {
      return;
    }

    if (event.key.toLowerCase() === 'i') {
      setRangeMarker('start', $editor.currentTime);
      return;
    }

    if (event.key.toLowerCase() === 'o') {
      setRangeMarker('end', $editor.currentTime);
      return;
    }

    if (event.code === 'Space') {
      event.preventDefault();
      preview?.togglePlayback();
      return;
    }

    if (event.key.toLowerCase() === 'j') {
      event.preventDefault();
      seekBySequence(-1);
      return;
    }

    if (event.key.toLowerCase() === 'k') {
      event.preventDefault();
      preview?.pausePlayback();
      return;
    }

    if (event.key.toLowerCase() === 'l') {
      event.preventDefault();
      seekBySequence(1);
      return;
    }

    if (event.key === '[' && canUseRange && normalizedRange) {
      event.preventDefault();
      seekTo(normalizedRange.start);
      return;
    }

    if (event.key === ']' && canUseRange && normalizedRange) {
      event.preventDefault();
      seekTo(normalizedRange.end);
      return;
    }

    if (event.key === ',' || event.key === '<') {
      event.preventDefault();
      goToPreviousMarker();
      return;
    }

    if (event.key === '.' || event.key === '>') {
      event.preventDefault();
      goToNextMarker();
      return;
    }

    if (event.shiftKey && event.key.toLowerCase() === 'm') {
      event.preventDefault();
      if (removeNearestBookmark($editor.currentTime)) {
        editor.update((state) => ({
          ...state,
          exportStatus: {
            state: 'idle',
            message: 'Removed marker at playhead.',
          },
        }));
      }
      return;
    }

    if (event.key.toLowerCase() === 'm') {
      event.preventDefault();
      addBookmarkMarker($editor.currentTime);
      return;
    }

    if (event.key.toLowerCase() === 's') {
      splitAtCurrentTime();
      return;
    }

    if (event.key === 'Delete' || event.key === 'Backspace') {
      event.preventDefault();
      if (selectedBookmarkId) {
        deleteSelectedBookmark();
      } else {
        deleteSelectedSegment();
      }
      return;
    }

    if (event.shiftKey && event.key.toLowerCase() === 'l' && canUseRange) {
      toggleRangeLoop();
      return;
    }

    if (!event.ctrlKey && event.key.toLowerCase() === 'z' && canUseRange) {
      event.preventDefault();
      zoomToRange();
      return;
    }

    if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();

      const frameStep = metadata?.fps ? 1 / metadata.fps : 1 / 30;
      const step = event.shiftKey ? 5 : frameStep;
      seekBySequence(event.key === 'ArrowRight' ? step : -step);
    }
  }

  function defaultExportFileName(): string {
    if (!$editor.currentFile) {
      return 'cutdown.mp4';
    }

    const leaf = $editor.currentFile.split(/[\\/]/).pop() ?? 'clip.mp4';
    return leaf.replace(/(\.[^.]+)?$/i, '-cutdown.mp4');
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
    return joinOutputPath(defaultExportDirectory(), defaultExportFileName());
  }

  function syncExportDefaultsForClip(settings?: AppSettings | null): void {
    outputFileName = defaultExportFileName();
    outputDirectory = defaultExportDirectory(settings);
  }

  async function chooseOutput(): Promise<void> {
    const settings = await invoke<AppSettings>('get_settings');
    const defaultName = defaultExportFileName();
    const defaultDir = defaultExportDirectory(settings) || undefined;

    const selected = await save({
      defaultPath:
        outputPath ||
        (outputDirectory
          ? joinOutputPath(outputDirectory, outputFileName)
          : defaultDir
            ? `${defaultDir}\\${defaultName}`
            : defaultOutputPath()),
      filters: [{ name: 'MP4 Video', extensions: ['mp4'] }],
    });

    if (selected) {
      applyOutputPath(selected);
      if (outputDirectory) {
        await invoke('set_last_export_dir', { path: outputDirectory });
      }
    }
  }

  async function openExportModal(): Promise<void> {
    const settings = await invoke<AppSettings>('get_settings').catch(() => null);
    outputFileName = defaultExportFileName();

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

    if (exportBatchPerSegment && exportMode === 'sequence' && exportSegments.length > 1) {
      return buildPerSegmentJobs(outputDirectory, outputFileName, exportSegments);
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
    const presetLabel = preset?.name ?? exportPresetId;

    exportProgressPercent = 0;
    statusDismissed = false;
    exportJobLabel = job.label;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message: `Exporting ${job.label} with ${presetLabel}...`,
        outputPath: job.outputPath,
      },
    }));

    const crop = exportCropPixels();
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
      },
    });

    exportProgressPercent = 100;
    lastShareUrl = null;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'success',
        message: `Exported ${job.label}: ${formatBytes(result.fileSize)} in ${formatTime(result.duration)}.`,
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
      await invoke('set_last_preset_id', { presetId: exportPresetId });
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

  async function exportClip(): Promise<void> {
    outputFileName = sanitizeExportFileName(outputFileName);

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

  function needsProxyPreview(meta: VideoMetadata): boolean {
    const codec = meta.codec.toLowerCase();
    return (
      meta.fileSize > 500_000_000 ||
      codec.includes('hevc') ||
      codec.includes('h265') ||
      codec.includes('vp9') ||
      codec.includes('av1')
    );
  }

  async function prepareProxyPreview(): Promise<void> {
    if (!$editor.currentFile || previewFallbackRunning) {
      return;
    }

    previewFallbackRunning = true;
    editor.update((state) => ({
      ...state,
      exportStatus: { state: 'running', message: 'Building proxy preview...' },
    }));

    try {
      const result = await invoke<PreviewResult>('prepare_preview', {
        params: { inputPath: $editor.currentFile, forceProxy: true },
      });
      await cleanupPreview($editor.previewTempPath);
      editor.update((state) => ({
        ...state,
        videoSrc: convertFileSrc(result.previewPath),
        previewTempPath: result.previewPath,
        previewStrategy: result.strategy as EditorState['previewStrategy'],
        exportStatus: {
          state: 'idle',
          message: `Proxy preview ready (${result.strategy}).`,
        },
      }));
    } catch (error) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: error instanceof Error ? error.message : String(error),
        },
      }));
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
        editor.update((state) => ({
          ...state,
          exportStatus: {
            state: 'error',
            message: 'Set a watch folder in Settings, or choose one when prompted.',
          },
        }));
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

  function currentProjectPayload(): Record<string, unknown> {
    return {
      version: 1,
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
    };
  }

  async function saveProject(): Promise<void> {
    if (!$editor.currentFile) {
      return;
    }

    const selected = await save({
      defaultPath: $editor.currentFile.replace(/\.[^.]+$/i, '.cutdown'),
      filters: [{ name: 'Cutdown project', extensions: ['cutdown'] }],
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
      filters: [{ name: 'Cutdown project', extensions: ['cutdown'] }],
    });

    if (!selected || typeof selected !== 'string') {
      return;
    }

    const project = await invoke<{
      sourcePath: string;
      segments: TimelineSegment[];
      selectedSegmentId: string | null;
      rangeStart: number | null;
      rangeEnd: number | null;
      cropEnabled: boolean;
      cropRect: NormalizedCropRect;
      clipVolume: number;
      currentTime: number | null;
      bookmarks: TimelineBookmark[];
      exportPresetId: string | null;
      accurateTrim: boolean;
      stripAudio: boolean;
    }>('load_project_file', { path: selected });

    if (!(await invoke<boolean>('path_exists', { path: project.sourcePath }))) {
      pendingProject = project;
      relinkMissingPath = project.sourcePath;
      relinkProjectName = selected.split(/[\\/]/).pop() ?? selected;
      relinkModalOpen = true;
      return;
    }

    await applyLoadedProject(project, selected);
  }

  async function applyLoadedProject(
    project: {
      sourcePath: string;
      segments: TimelineSegment[];
      selectedSegmentId: string | null;
      rangeStart: number | null;
      rangeEnd: number | null;
      cropEnabled: boolean;
      cropRect: NormalizedCropRect;
      clipVolume: number;
      currentTime: number | null;
      bookmarks: TimelineBookmark[];
      exportPresetId: string | null;
      accurateTrim: boolean;
      stripAudio: boolean;
    },
    projectLabel: string,
  ): Promise<void> {
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

    editor.update((state) => ({
      ...state,
      segments: project.segments,
      selectedSegmentId: project.selectedSegmentId,
      currentTime: clamp(project.currentTime ?? 0, 0, state.metadata?.duration ?? 0),
      exportStatus: { state: 'idle', message: `Loaded project ${projectLabel}.` },
    }));
    schedulePersistSession();
    await tick();
    seekTo(get(editor).currentTime);
    pushToast(`Loaded project ${projectLabel}.`, 'success');
  }

  async function relinkProjectSource(): Promise<void> {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: 'Video',
          extensions: ['mp4', 'mkv', 'mov', 'avi', 'webm', 'ts', 'flv'],
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
      return;
    }

    if ($editor.previewStrategy === 'Preview proxy') {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: 'The generated preview proxy could not be played.',
        },
      }));
      return;
    }

    previewFallbackRunning = true;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message: $editor.previewStrategy === 'Preview remux' ? 'Generating preview proxy...' : 'Trying preview remux...',
      },
    }));

    try {
      const previousPreview = $editor.previewTempPath;
      const result = await invoke<PreviewResult>('prepare_preview', {
        params: {
          inputPath: $editor.currentFile,
          forceProxy: $editor.previewStrategy === 'Preview remux',
        },
      });

      await cleanupPreview(previousPreview);
      editor.update((state) => ({
        ...state,
        videoSrc: convertFileSrc(result.previewPath),
        previewTempPath: result.previewPath,
        previewStrategy: result.strategy,
        currentTime: 0,
        exportStatus: {
          state: 'idle',
          message: `${result.strategy} ready. Export will still use the original file.`,
        },
      }));
      preview?.seekTo(0);
    } catch (error) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: error instanceof Error ? error.message : String(error),
        },
      }));
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
  on:click={handleWindowClick}
/>

<ToastHost />

<main class="shell" class:shell--dragover={dragOver}>
  {#if !ffmpegAvailable || !trayHintDismissed}
    <div class="shell__alerts">
      {#if !ffmpegAvailable}
        <div class="ffmpeg-banner" role="alert">
          <span>
            {ffmpegInstalling
              ? ffmpegInstallMessage || 'Downloading ffmpeg…'
              : ffmpegStatus ||
                'ffmpeg is required for export and clip analysis. Download it once (~80 MB) or install ffmpeg on PATH.'}
            {#if ffmpegInstalling && ffmpegInstallPercent !== null}
              {' '}({Math.round(ffmpegInstallPercent)}%)
            {/if}
          </span>
          <button
            type="button"
            class="secondary"
            disabled={ffmpegInstalling}
            on:click={() => void installFfmpeg()}
          >
            {ffmpegInstalling ? 'Downloading…' : 'Download ffmpeg'}
          </button>
          <button type="button" class="secondary" title="Open Settings" on:click={() => void openSettings()}>Settings</button>
        </div>
      {/if}
      {#if !trayHintDismissed}
        <div class="tray-hint-banner">
          <span>Closing the window minimizes Cutdown to the system tray. Use the tray icon or Open Editor to restore.</span>
          <button type="button" class="secondary" on:click={dismissTrayHint}>Dismiss</button>
        </div>
      {/if}
    </div>
  {/if}

  <section class="toolbar" aria-label="Editor toolbar">
    <IconButton icon="open" title="Open video file" on:click={chooseClip} />
    <div class="toolbar-recent">
      <button
        type="button"
        class="secondary"
        class:active={recentMenuOpen}
        title="Recently opened sources"
        aria-expanded={recentMenuOpen}
        on:click|stopPropagation={toggleRecentMenu}
      >
        Recent
      </button>
      {#if recentMenuOpen}
        <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
        <div class="toolbar-recent__menu" role="menu" tabindex="-1" on:click|stopPropagation={() => {}}>
          {#if recentSources.length === 0}
            <p class="toolbar-recent__empty">No recent sources yet.</p>
          {:else}
            <ul class="toolbar-recent__list">
              {#each recentSources as source (source)}
                <li>
                  <button type="button" role="menuitem" title={source} on:click={() => openRecentSource(source)}>
                    {source.split(/[\\/]/).pop() ?? source}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      {/if}
    </div>
    <IconButton icon="save" title="Save Cutdown project" disabled={!$editor.currentFile} on:click={() => void saveProject()} />
    <button type="button" class="secondary" title="Open Cutdown project" on:click={() => void openProject()}>Open project</button>
    <button type="button" class="secondary" title="Open newest video in watch folder" on:click={() => void loadLatestReplay()}>Latest replay</button>
    <IconButton icon="undo" title="Undo (Ctrl+Z)" disabled={segmentHistory.length === 0} on:click={undoSegmentEdit} />
    <IconButton icon="redo" title="Redo (Ctrl+Y)" disabled={redoHistory.length === 0} on:click={redoSegmentEdit} />
    <div class="toolbar__spacer"></div>
    {#if $editor.currentFile}
      <span class="toolbar__clip-name" title={$editor.currentFile}>{fileName}</span>
    {/if}
    <span class="toolbar__status">{ffmpegAvailable ? 'Ready' : 'ffmpeg missing'}</span>
    <button type="button" class="tool-button" title="Help — shortcuts and features (?)" on:click={() => (shortcutsModalOpen = true)}>?</button>
    <IconButton icon="history" title="Clip history" on:click={() => (historyDrawerOpen = true)} />
    <IconButton icon="settings" title="Settings" on:click={() => void openSettings()} />
    <IconButton icon="export" title="Export clip" variant="primary" showLabel disabled={!canExport || openingClip} on:click={openExportModal} />
  </section>

  <ExportActivity
    visible={exportActivityVisible}
    jobLabel={exportJobLabel}
    jobIndex={exportJobIndex}
    jobTotal={exportJobTotal}
    presetName={exportPresetDisplayName}
    percent={exportProgressPercent}
    message={$editor.exportStatus.message}
    outputPath={$editor.exportStatus.outputPath ?? null}
    on:reveal={revealExport}
  />

  <section class="editor-workspace" bind:this={workspaceEl}>
  <section class="preview-panel" style={`flex: ${workspaceSplitRatio} 1 0`}>
    {#if openingClip}
      <div class="opening-overlay" aria-busy="true"><span>Opening clip…</span></div>
    {/if}
    <div class="preview-panel__tools">
      <IconButton
        icon="crop"
        title={cropEnabled ? 'Disable crop overlay' : 'Enable crop overlay'}
        variant="secondary"
        active={cropEnabled}
        disabled={!$editor.currentFile}
        on:click={() => {
          cropEnabled = !cropEnabled;
          schedulePersistSession();
        }}
      />
      <label class="preview-panel__crop-lock" title="Lock crop aspect ratio while resizing">
        <input type="checkbox" bind:checked={cropLockAspect} disabled={!cropEnabled} />
        Lock ratio
      </label>
      <button
        type="button"
        class="secondary"
        class:active={cropAspect === '16:9'}
        disabled={!cropEnabled}
        title="Crop 16:9"
        on:click={() => applyAspectCrop('16:9')}
      >
        16:9
      </button>
      <button
        type="button"
        class="secondary"
        class:active={cropAspect === '9:16'}
        disabled={!cropEnabled}
        title="Crop 9:16"
        on:click={() => applyAspectCrop('9:16')}
      >
        9:16
      </button>
      <button
        type="button"
        class="secondary"
        class:active={cropAspect === '4:3'}
        disabled={!cropEnabled}
        title="Crop 4:3"
        on:click={() => applyAspectCrop('4:3')}
      >
        4:3
      </button>
      <button
        type="button"
        class="secondary"
        class:active={cropAspect === '1:1'}
        disabled={!cropEnabled}
        title="Crop 1:1"
        on:click={() => applyAspectCrop('1:1')}
      >
        1:1
      </button>
      <span class="preview-panel__crop-custom" class:preview-panel__crop-custom--disabled={!cropEnabled}>
        <input
          type="number"
          class="preview-panel__crop-input"
          min="1"
          max="100"
          bind:value={cropCustomAspectW}
          disabled={!cropEnabled}
          aria-label="Custom aspect width"
        />
        <span>:</span>
        <input
          type="number"
          class="preview-panel__crop-input"
          min="1"
          max="100"
          bind:value={cropCustomAspectH}
          disabled={!cropEnabled}
          aria-label="Custom aspect height"
        />
        <button
          type="button"
          class="secondary"
          class:active={cropAspect === 'custom'}
          disabled={!cropEnabled}
          title="Apply custom aspect ratio"
          on:click={() => applyAspectCrop('custom')}
        >
          Custom
        </button>
      </span>
      <button
        type="button"
        class="secondary"
        class:active={cropAspect === 'free'}
        disabled={!cropEnabled}
        title="Free crop (unlocked aspect)"
        on:click={() => applyAspectCrop('free')}
      >
        Free
      </button>
      <span class="preview-panel__divider" aria-hidden="true"></span>
      <IconButton
        icon="scaleFit"
        title="Fit preview to panel"
        variant="secondary"
        disabled={!$editor.currentFile}
        on:click={() => preview?.fitToView()}
      />
      <button
        type="button"
        class="secondary"
        title="Zoom preview out"
        disabled={!$editor.currentFile}
        on:click={() => preview?.zoomOut()}
      >
        −
      </button>
      <button
        type="button"
        class="secondary"
        title="Zoom preview in"
        disabled={!$editor.currentFile}
        on:click={() => preview?.zoomIn()}
      >
        +
      </button>
      <span class="preview-panel__divider" aria-hidden="true"></span>
      <button type="button" class="secondary" class:active={previewPlaybackRate === 0.5} disabled={!$editor.currentFile} on:click={() => (previewPlaybackRate = 0.5)}>0.5×</button>
      <button type="button" class="secondary" class:active={previewPlaybackRate === 1} disabled={!$editor.currentFile} on:click={() => (previewPlaybackRate = 1)}>1×</button>
      <button type="button" class="secondary" class:active={previewPlaybackRate === 2} disabled={!$editor.currentFile} on:click={() => (previewPlaybackRate = 2)}>2×</button>
      <button type="button" class="secondary" title="Build proxy preview for heavy codecs" disabled={!$editor.currentFile || previewFallbackRunning} on:click={() => void prepareProxyPreview()}>Proxy</button>
    </div>
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
        schedulePersistSession();
      }}
      on:metadata={() => {}}
      on:previewready={handlePreviewReady}
      on:error={(event) => void handlePreviewError(event.detail.message)}
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

  <div class="timeline-pane" style={`flex: ${1 - workspaceSplitRatio} 1 0`}>
    <div class="timeline-pane__tools">
      <IconButton icon="split" title="Split at playhead (S)" disabled={!canExport} on:click={splitAtCurrentTime} />
      <span class="timeline-pane__divider" aria-hidden="true"></span>
      <IconButton
        icon="markIn"
        title="Mark In at playhead (I)"
        variant="secondary"
        disabled={!canExport}
        on:click={() => setRangeMarker('start', $editor.currentTime)}
      />
      <IconButton
        icon="markOut"
        title="Mark Out at playhead (O)"
        variant="secondary"
        disabled={!canExport}
        on:click={() => setRangeMarker('end', $editor.currentTime)}
      />
      <IconButton
        icon="clearRange"
        title="Clear I/O range"
        variant="secondary"
        disabled={!canUseRange}
        on:click={clearRange}
      />
      <span class="timeline-pane__divider" aria-hidden="true"></span>
      <IconButton
        icon="bookmark"
        title="Add marker at playhead (M)"
        variant="secondary"
        disabled={!canExport}
        on:click={() => addBookmarkMarker($editor.currentTime)}
      />
      <button
        type="button"
        class="secondary"
        title="Previous marker (,)"
        disabled={!bookmarks.length}
        on:click={goToPreviousMarker}
      >
        ‹
      </button>
      <button
        type="button"
        class="secondary"
        title="Next marker (.)"
        disabled={!bookmarks.length}
        on:click={goToNextMarker}
      >
        ›
      </button>
      <IconButton
        icon="delete"
        title="Delete selected marker (Del)"
        variant="secondary"
        disabled={!selectedBookmarkId}
        on:click={deleteSelectedBookmark}
      />
      <span class="timeline-pane__divider" aria-hidden="true"></span>
      <IconButton
        icon="snapIn"
        title="Snap playhead to In ( [ )"
        variant="secondary"
        disabled={!canUseRange}
        on:click={snapToRangeStart}
      />
      <IconButton
        icon="snapOut"
        title="Snap playhead to Out ( ] )"
        variant="secondary"
        disabled={!canUseRange}
        on:click={snapToRangeEnd}
      />
      <span class="timeline-pane__divider" aria-hidden="true"></span>
      <button
        type="button"
        class="secondary"
        title="Split at I/O markers"
        disabled={!canUseRange}
        on:click={splitAtRangeMarkers}
      >
        Split I/O
      </button>
      <IconButton
        icon="keepRange"
        title="Keep only I/O range — remove all footage outside In/Out"
        variant="secondary"
        disabled={!canUseRange}
        on:click={keepOnlyRange}
      />
      <button
        type="button"
        class="secondary"
        title="Deselect segment (Esc)"
        disabled={!selectedSegment}
        on:click={deselectSegment}
      >
        Deselect
      </button>
      <IconButton
        icon="delete"
        title="Delete selected segment (Del)"
        variant="secondary"
        disabled={!selectedSegment || segments.length <= 1}
        on:click={deleteSelectedSegment}
      />
    </div>
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
    sourceDuration={metadata?.duration ?? 0}
    selectedSegmentId={$editor.selectedSegmentId}
    rangeStart={normalizedRange?.start ?? null}
    rangeEnd={normalizedRange?.end ?? null}
    zoom={timelineZoom}
    videoTrackHeight={videoTrackHeight}
    audioTrackHeight={audioTrackHeight}
    audioCodec={metadata?.audioCodec ?? null}
    audioChannels={metadata?.audioChannels ?? null}
    on:seek={(event) => seekTo(event.detail.seconds)}
    on:selectSegment={(event) => selectSegment(event.detail.id)}
    on:rangeChange={(event) => {
      rangeStart = event.detail.start;
      rangeEnd = event.detail.end;
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
    on:reorderSegment={(event) => reorderSegment(event.detail.id, event.detail.toIndex)}
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

  <section class="transport-bar" aria-label="Editor controls">
    <div class="transport-bar__stat">
      <span>Selected</span>
      <strong>{selectedSegment ? formatTime(selectedSegment.sourceEnd - selectedSegment.sourceStart) : 'None'}</strong>
      {#if selectedSegment}
        <small>{formatTime(selectedSegment.sourceStart)} - {formatTime(selectedSegment.sourceEnd)}</small>
      {/if}
    </div>
    <div class="transport-bar__stat">
      <span>Range</span>
      <strong>{normalizedRange ? formatTime(rangeDuration) : 'None'}</strong>
      {#if normalizedRange}
        <small>{formatTime(normalizedRange.start)} - {formatTime(normalizedRange.end)}</small>
      {/if}
    </div>
    <div class="transport-bar__stat">
      <span>Output</span>
      <strong>{formatTime(outputDuration)}</strong>
      <small>{segments.length} segment{segments.length === 1 ? '' : 's'}</small>
    </div>
    <div class="transport-bar__volume">
      <label for="clip-volume">Volume</label>
      <input
        id="clip-volume"
        class="app-slider"
        type="range"
        min="0"
        max="100"
        step="1"
        value={Math.round(clipVolume * 100)}
        disabled={!$editor.currentFile || !metadata?.audioCodec}
        aria-label="Clip volume"
        style={`--slider-fill: ${Math.round(clipVolume * 100)}%`}
        on:input={handleVolumeInput}
      />
      <span>{Math.round(clipVolume * 100)}%</span>
    </div>
    <div class="transport-bar__actions">
      <IconButton
        icon="loop"
        variant="secondary"
        active={rangeLoopPlayback}
        title={rangeLoopPlayback ? 'Disable range loop (Shift+L)' : 'Loop playback in I/O range (Shift+L)'}
        disabled={!canUseRange}
        on:click={toggleRangeLoop}
      />
      <button type="button" class="secondary" title="Delete selected segment (Del)" disabled={!selectedSegment || segments.length <= 1} on:click={deleteSelectedSegment}>Delete</button>
    </div>
  </section>

  <footer class="bottom-bar">
    <div class="metadata">
      {#if metadata}
        <span>{metadata.width}x{metadata.height}</span>
        <span>{metadata.fps.toFixed(2)} fps</span>
        <span>{metadata.codec}</span>
        <span>{metadata.audioCodec ? `${metadata.audioCodec}${metadata.audioChannels ? ` ${metadata.audioChannels}ch` : ''}` : 'no audio'}</span>
        <span>{$editor.previewStrategy}</span>
        <span>{formatBytes(metadata.fileSize)}</span>
      {:else}
        <span>Open a clip to start cutting</span>
      {/if}
    </div>
    <div class="bottom-bar__status">
      <ProgressBar
        active={$editor.exportStatus.state === 'running'}
        label={$editor.exportStatus.message}
        percent={exportProgressPercent}
        state={$editor.exportStatus.state}
        dismissible={statusDismissible}
        on:dismiss={dismissExportStatus}
      />
      {#if $editor.exportStatus.outputPath && ($editor.exportStatus.state === 'success' || $editor.exportStatus.state === 'error')}
        <div class="bottom-bar__result">
          <span class="bottom-bar__result-name" title={$editor.exportStatus.outputPath}>
            {$editor.exportStatus.outputPath.split(/[\\/]/).pop()}
          </span>
          <button
            type="button"
            class="secondary"
            title="Copy output file path"
            on:click={() => void invoke('copy_text_to_clipboard', { text: $editor.exportStatus.outputPath ?? '' })}
          >
            Copy path
          </button>
          {#if lastShareUrl}
            <button type="button" class="secondary" title="Copy share link" on:click={() => void copyShareLink(lastShareUrl ?? '')}>
              Copy link
            </button>
          {/if}
          <button type="button" class="secondary" title="Show exported file in Explorer" on:click={revealExport}>Open folder</button>
          <button type="button" class="secondary" title="Upload and copy share link" on:click={() => openUploadPicker($editor.exportStatus.outputPath ?? '')}>
            Upload
          </button>
          <button type="button" class="secondary" title="Open export settings" on:click={openExportModal}>Export again</button>
        </div>
      {:else if $editor.exportStatus.outputPath && $editor.exportStatus.state === 'running'}
        <button type="button" class="secondary" title="Cancel export" on:click={() => void cancelExport()}>Cancel</button>
        <button type="button" class="secondary" title="Show exported file in Explorer" on:click={revealExport}>Open folder</button>
      {/if}
    </div>
  </footer>
</main>

<ExportModal
  open={exportModalOpen}
  {outputDirectory}
  bind:outputFileName
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
  message="Replace the current clip with the new watch-folder file? Unsaved segment edits will be lost."
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
  bind:uploadProviders
  bind:defaultUploadProviderId
  bind:customExportPresets
  {ffmpegStatus}
  {gpuEncoders}
  on:close={() => (settingsModalOpen = false)}
  on:restoreTrayHint={restoreTrayHint}
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
    customExportPresets = event.detail.customExportPresets;
    try {
      await reloadUploadProvidersFromDisk();
    } catch (error) {
      pushToast(error instanceof Error ? error.message : String(error), 'error');
    }
    void refreshExportPresets();
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

<ClipHistoryDrawer
  open={historyDrawerOpen}
  entries={clipHistory}
  busyPath={historyBusyPath}
  uploadConfigured={uploadTargetsConfigured}
  on:close={() => (historyDrawerOpen = false)}
  on:reveal={(event) => void invoke('reveal_in_explorer', { path: event.detail.path })}
  on:openClip={(event) => void openClipPath(event.detail.path)}
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
