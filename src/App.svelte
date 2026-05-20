<script lang="ts">
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { confirm, open, save } from '@tauri-apps/plugin-dialog';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import ClipHistoryDrawer from './components/ClipHistoryDrawer.svelte';
  import ExportModal from './components/ExportModal.svelte';
  import IconButton from './components/IconButton.svelte';
  import ProgressBar from './components/ProgressBar.svelte';
  import SettingsModal from './components/SettingsModal.svelte';
  import Timeline from './components/Timeline.svelte';
  import VideoPreview from './components/VideoPreview.svelte';
  import {
    clamp,
    formatBytes,
    formatTime,
    joinOutputPath,
    sanitizeExportFileName,
    splitOutputPath,
  } from './lib/format';
  import type { ClipHistoryEntry, NormalizedCropRect } from './lib/types';
  import {
    createFullSegment,
    editor,
    totalSegmentDuration,
    type TimelineSegment,
    type VideoMetadata,
  } from './stores/editor';

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
  };

  type PreviewResult = {
    previewPath: string;
    strategy: 'Preview remux' | 'Preview proxy';
  };

  let preview:
    | {
        seekTo(seconds: number): void;
        togglePlayback(): void;
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
  let historyDrawerOpen = false;
  let clipHistory: ClipHistoryEntry[] = [];
  let historyBusyPath: string | null = null;
  let cropEnabled = false;
  let cropAspect: 'free' | '16:9' | '9:16' = 'free';
  let cropRect: NormalizedCropRect = { x: 0.05, y: 0.05, width: 0.9, height: 0.9 };
  let catboxUserHash = '';
  let catboxApiUrl = '';
  let exportPresets: PresetInfo[] = [];
  let exportPresetId = 'lossless-trim';
  let gpuEncoders: string[] = [];
  let preferGpuEncoding = true;
  let defaultExportDir: string | null = null;
  let runAtStartup = false;
  let exportMode: 'sequence' | 'range' = 'sequence';
  let rangeLoopPlayback = false;
  let clipVolume = 1;
  let exportProgressPercent: number | null = null;
  let outputPath = '';
  let outputDirectory = '';
  let outputFileName = 'cutdown.mp4';
  let watchFolder: string | null = null;
  let watchFolderEnabled = false;
  let ffmpegStatus = '';
  let ffmpegAvailable = true;
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
  let rangeStart: number | null = null;
  let rangeEnd: number | null = null;
  let currentWindowTitle = '';

  onMount(() => {
    void bootstrapApp();

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

    return () => {
      void unlistenExport.then((stop) => stop());
      void unlistenWatch.then((stop) => stop());
      const previewTempPath = get(editor).previewTempPath;
      if (previewTempPath) {
        void invoke('cleanup_preview', { path: previewTempPath });
      }
    };
  });

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
      catboxUserHash = settings.catboxUserHash ?? '';
      catboxApiUrl = settings.catboxApiUrl ?? '';
      exportPresets = presets;
      clipHistory = await invoke<ClipHistoryEntry[]>('list_clip_history');
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
      const proceed = await confirm('Replace the current clip with the new watch-folder file?', {
        title: 'Open new clip',
        kind: 'warning',
      });

      if (!proceed) {
        return;
      }
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
  $: normalizedRange =
    rangeStart !== null && rangeEnd !== null
      ? {
          start: Math.min(rangeStart, rangeEnd),
          end: Math.max(rangeStart, rangeEnd),
        }
      : null;
  $: rangeDuration = normalizedRange ? normalizedRange.end - normalizedRange.start : 0;
  $: canUseRange = Boolean(normalizedRange && rangeDuration > 0.05);
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
      exportStatus: {
        state: 'idle',
        message: 'Undid last edit.',
      },
    }));
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
      exportStatus: {
        state: 'idle',
        message: 'Redid edit.',
      },
    }));
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

    await cleanupPreview($editor.previewTempPath);
    segmentHistory = [];
    redoHistory = [];
    rangeStart = null;
    rangeEnd = null;
    rangeLoopPlayback = false;
    clipVolume = 1;
    exportMode = 'sequence';
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
      syncExportDefaultsForClip();
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

  function seekTo(seconds: number): void {
    const nextTime = clamp(seconds, 0, duration);
    preview?.seekTo(nextTime);
    editor.update((state) => ({ ...state, currentTime: nextTime }));
  }

  function selectSegment(id: string | null): void {
    editor.update((state) => ({ ...state, selectedSegmentId: id }));
  }

  function splitAtTime(splitTime: number): void {
    if (!canExport) {
      return;
    }

    const targetSegment = $editor.segments.find(
      (segment) => splitTime > segment.sourceStart + 0.05 && splitTime < segment.sourceEnd - 0.05,
    );

    if (!targetSegment) {
      return;
    }

    pushUndoSnapshot();
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
      exportStatus: {
        state: 'idle',
        message: `Split at ${formatTime(splitTime)}.`,
      },
    }));
  }

  function splitAtCurrentTime(): void {
    splitAtTime($editor.currentTime);
  }

  function deleteSelectedSegment(): void {
    if (!$editor.selectedSegmentId) {
      return;
    }

    if ($editor.segments.length <= 1) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'idle',
          message: 'At least one segment is required.',
        },
      }));
      return;
    }

    pushUndoSnapshot();
    editor.update((state) => ({
      ...state,
      segments: state.segments.filter((segment) => segment.id !== state.selectedSegmentId),
      selectedSegmentId: null,
      exportStatus: {
        state: 'idle',
        message: 'Deleted selected segment.',
      },
    }));
  }

  function splitAtRangeMarkers(): void {
    if (!canUseRange || !normalizedRange) {
      return;
    }

    pushUndoSnapshot();
    splitAtTime(normalizedRange.start);
    splitAtTime(normalizedRange.end);
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
  }

  function handleVolumeInput(event: Event): void {
    const value = Number((event.currentTarget as HTMLInputElement).value);
    clipVolume = clamp(value / 100, 0, 1);
  }

  function toggleRangeLoop(): void {
    if (!canUseRange) {
      return;
    }

    rangeLoopPlayback = !rangeLoopPlayback;

    if (rangeLoopPlayback && normalizedRange) {
      seekTo(normalizedRange.start);
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
  }

  function clearRange(): void {
    rangeStart = null;
    rangeEnd = null;
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
    historyDrawerOpen = false;
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

  function applyAspectCrop(aspect: 'free' | '16:9' | '9:16'): void {
    cropAspect = aspect;

    if (aspect === 'free' || !metadata) {
      return;
    }

    const frameRatio = metadata.width / metadata.height;
    const targetRatio = aspect === '16:9' ? 16 / 9 : 9 / 16;
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
  }

  async function clearHistory(): Promise<void> {
    await invoke('clear_clip_history');
    clipHistory = [];
  }

  async function uploadClip(path: string): Promise<void> {
    historyBusyPath = path;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message: 'Uploading to Catbox...',
      },
    }));

    try {
      const url = await invoke<string>('upload_to_catbox', { filePath: path });
      await invoke('copy_text_to_clipboard', { text: url });
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'success',
          message: `Upload complete. Link copied: ${url}`,
          outputPath: state.exportStatus.outputPath,
          outputSize: state.exportStatus.outputSize,
        },
      }));
      await notify('Upload link copied to clipboard.', 'Catbox upload');
    } catch (error) {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: error instanceof Error ? error.message : String(error),
        },
      }));
    } finally {
      historyBusyPath = null;
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
      if (exportModalOpen || settingsModalOpen || historyDrawerOpen) {
        event.preventDefault();
        closeModals();
        return;
      }
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

    if (event.key.toLowerCase() === 's') {
      splitAtCurrentTime();
      return;
    }

    if (event.key === 'Delete' || event.key === 'Backspace') {
      deleteSelectedSegment();
      return;
    }

    if (event.key.toLowerCase() === 'l' && canUseRange) {
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
      seekTo($editor.currentTime + (event.key === 'ArrowRight' ? step : -step));
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

  async function exportClip(): Promise<void> {
    outputFileName = sanitizeExportFileName(outputFileName);

    if (!$editor.currentFile || !outputPath || !outputDirectory) {
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

    try {
      const exists = await invoke<boolean>('path_exists', { path: outputPath }).catch(() => false);
      if (exists) {
        const overwrite = await confirm(`Replace existing file?\n${outputPath}`, {
          title: 'Overwrite output',
          kind: 'warning',
        });

        if (!overwrite) {
          return;
        }
      }
    } catch {
      // Overwrite confirmation is best-effort when the helper is unavailable.
    }

    const exportSegments =
      exportMode === 'range' && canUseRange && normalizedRange
        ? [{ start: normalizedRange.start, end: normalizedRange.end }]
        : segments.map((segment) => ({
            start: segment.sourceStart,
            end: segment.sourceEnd,
          }));

    if (exportSegments.length === 0) {
      return;
    }

    const preset = exportPresets.find((item) => item.id === exportPresetId);
    const presetLabel = preset?.name ?? exportPresetId;

    exportProgressPercent = 0;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message: `Exporting with ${presetLabel}...`,
      },
    }));

    try {
      const crop = exportCropPixels();
      const result = await invoke<ExportResult>('export_clip', {
        params: {
          inputPath: $editor.currentFile,
          outputPath,
          audioMode: 'preserve',
          segments: exportSegments,
          presetId: exportPresetId,
          preferGpu: preferGpuEncoding,
          sourcePath: $editor.currentFile,
          crop,
          volume: clipVolume,
        },
      });

      exportModalOpen = false;
      await refreshClipHistory();
      exportProgressPercent = 100;
      await invoke('set_last_preset_id', { presetId: exportPresetId });
      const parent = result.outputPath.replace(/[\\/][^\\/]+$/, '');
      if (parent) {
        await invoke('set_last_export_dir', { path: parent });
      }

      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'success',
          message: `Exported ${formatBytes(result.fileSize)} in ${formatTime(result.duration)}.`,
          outputPath: result.outputPath,
          outputSize: result.fileSize,
        },
      }));
      await notify(`Exported ${formatBytes(result.fileSize)} clip.`, 'Export complete');
    } catch (error) {
      exportProgressPercent = null;
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: error instanceof Error ? error.message : String(error),
        },
      }));
    }
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

<svelte:window on:keydown={handleKeydown} />

<main class="shell">
  <section class="toolbar" aria-label="Editor toolbar">
    <IconButton icon="open" title="Open video file" on:click={chooseClip} />
    <IconButton icon="undo" title="Undo (Ctrl+Z)" disabled={segmentHistory.length === 0} on:click={undoSegmentEdit} />
    <IconButton icon="redo" title="Redo (Ctrl+Y)" disabled={redoHistory.length === 0} on:click={redoSegmentEdit} />
    <IconButton icon="split" title="Split at playhead (S)" disabled={!canExport} on:click={splitAtCurrentTime} />
    <div class="toolbar__spacer"></div>
    <span class="toolbar__status">{ffmpegAvailable ? 'Ready' : 'ffmpeg missing'}</span>
    <IconButton icon="history" title="Clip history" on:click={() => (historyDrawerOpen = true)} />
    <IconButton icon="settings" title="Settings" on:click={() => (settingsModalOpen = true)} />
    <IconButton icon="export" title="Export clip" variant="primary" showLabel disabled={!canExport} on:click={openExportModal} />
  </section>

  <section class="preview-panel">
    <div class="preview-panel__tools">
      <IconButton
        icon="crop"
        title={cropEnabled ? 'Disable crop overlay' : 'Enable crop overlay'}
        variant="secondary"
        active={cropEnabled}
        disabled={!$editor.currentFile}
        on:click={() => (cropEnabled = !cropEnabled)}
      />
      <button type="button" class="secondary" disabled={!cropEnabled} title="Crop to 16:9 aspect" on:click={() => applyAspectCrop('16:9')}>16:9</button>
      <button type="button" class="secondary" disabled={!cropEnabled} title="Crop to 9:16 aspect" on:click={() => applyAspectCrop('9:16')}>9:16</button>
      <button type="button" class="secondary" disabled={!cropEnabled} title="Free crop aspect" on:click={() => applyAspectCrop('free')}>Free</button>
    </div>
    <VideoPreview
      bind:this={preview}
      src={$editor.videoSrc}
      currentTime={$editor.currentTime}
      loopEnabled={rangeLoopPlayback}
      loopStart={normalizedRange?.start ?? null}
      loopEnd={normalizedRange?.end ?? null}
      volume={clipVolume}
      {cropEnabled}
      bind:cropRect
      on:cropChange={(event) => (cropRect = event.detail.rect)}
      on:metadata={() => {}}
      on:error={(event) => void handlePreviewError(event.detail.message)}
      on:timeupdate={(event) => {
        editor.update((state) => ({ ...state, currentTime: event.detail.currentTime }));
      }}
    />
  </section>

  <Timeline
    bind:this={timeline}
    disabled={!$editor.currentFile}
    {duration}
    currentTime={$editor.currentTime}
    {segments}
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
  />

  <section class="transport-bar" aria-label="Editor controls">
    <div>
      <span>Selected</span>
      <strong>{selectedSegment ? formatTime(selectedSegment.sourceEnd - selectedSegment.sourceStart) : 'None'}</strong>
      {#if selectedSegment}
        <small>{formatTime(selectedSegment.sourceStart)} - {formatTime(selectedSegment.sourceEnd)}</small>
      {/if}
    </div>
    <div>
      <span>Range</span>
      <strong>{normalizedRange ? formatTime(rangeDuration) : 'None'}</strong>
      {#if normalizedRange}
        <small>{formatTime(normalizedRange.start)} - {formatTime(normalizedRange.end)}</small>
      {/if}
    </div>
    <div>
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
      <button type="button" class="secondary" title="Split at I/O markers" disabled={!canUseRange} on:click={splitAtRangeMarkers}>Split I/O</button>
      <button type="button" class="secondary" title="Keep only I/O range" disabled={!canUseRange} on:click={keepOnlyRange}>Keep range</button>
      <button type="button" class="secondary" title="Trim outside I/O range" disabled={!canUseRange} on:click={deleteOutsideRange}>Trim outside</button>
      <IconButton
        icon="loop"
        variant="secondary"
        active={rangeLoopPlayback}
        title={rangeLoopPlayback ? 'Disable range loop (L)' : 'Loop playback in I/O range (L)'}
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
      />
      {#if $editor.exportStatus.outputPath}
        <button type="button" class="secondary" title="Show exported file in Explorer" on:click={revealExport}>Open Folder</button>
        <button
          type="button"
          class="secondary"
          title="Upload to Catbox and copy link"
          disabled={historyBusyPath === $editor.exportStatus.outputPath}
          on:click={() => void uploadClip($editor.exportStatus.outputPath ?? '')}
        >
          Upload
        </button>
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
  on:close={() => (exportModalOpen = false)}
  on:chooseOutput={chooseOutput}
  on:exportModeChange={(event) => (exportMode = event.detail.mode)}
  on:presetChange={(event) => (exportPresetId = event.detail.presetId)}
  on:confirm={exportClip}
/>

<SettingsModal
  visible={settingsModalOpen}
  {watchFolder}
  {watchFolderEnabled}
  {defaultExportDir}
  {exportPresetId}
  {preferGpuEncoding}
  {runAtStartup}
  {catboxUserHash}
  {catboxApiUrl}
  {ffmpegStatus}
  {gpuEncoders}
  on:close={() => (settingsModalOpen = false)}
  on:saved={(event) => {
    watchFolder = event.detail.watchFolder;
    watchFolderEnabled = event.detail.watchFolderEnabled;
    defaultExportDir = event.detail.defaultExportDir;
    exportPresetId = event.detail.lastPresetId;
    preferGpuEncoding = event.detail.preferGpuEncoding;
    runAtStartup = event.detail.runAtStartup;
    catboxUserHash = event.detail.catboxUserHash;
    catboxApiUrl = event.detail.catboxApiUrl;
  }}
/>

<ClipHistoryDrawer
  open={historyDrawerOpen}
  entries={clipHistory}
  busyPath={historyBusyPath}
  on:close={() => (historyDrawerOpen = false)}
  on:reveal={(event) => void invoke('reveal_in_explorer', { path: event.detail.path })}
  on:openClip={(event) => void openClipPath(event.detail.path)}
  on:copyPath={(event) => void invoke('copy_text_to_clipboard', { text: event.detail.path })}
  on:upload={(event) => void uploadClip(event.detail.path)}
  on:remove={(event) => void removeHistoryEntry(event.detail.path)}
  on:clear={() => void clearHistory()}
/>
