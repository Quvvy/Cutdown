<script lang="ts">
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { confirm, open, save } from '@tauri-apps/plugin-dialog';
  import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import ExportModal from './components/ExportModal.svelte';
  import ProgressBar from './components/ProgressBar.svelte';
  import SettingsModal from './components/SettingsModal.svelte';
  import Timeline from './components/Timeline.svelte';
  import VideoPreview from './components/VideoPreview.svelte';
  import { clamp, formatBytes, formatTime } from './lib/format';
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

  type AppSettings = {
    watchFolder: string | null;
    watchFolderEnabled: boolean;
    lastExportDir: string | null;
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
      }
    | undefined;
  let exportModalOpen = false;
  let settingsModalOpen = false;
  let exportMode: 'sequence' | 'range' = 'sequence';
  let rangeLoopPlayback = false;
  let exportProgressPercent: number | null = null;
  let outputPath = '';
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
      const [settings, ffmpeg] = await Promise.all([
        invoke<AppSettings>('get_settings'),
        invoke<FfmpegCheckResult>('check_ffmpeg'),
      ]);

      watchFolder = settings.watchFolder;
      watchFolderEnabled = settings.watchFolderEnabled;
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
  $: updateWindowTitle(fileName);

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
    timelineZoom = 0;
  }

  function handleKeydown(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    const isTextInput =
      target?.tagName === 'TEXTAREA' ||
      target?.isContentEditable ||
      (target instanceof HTMLInputElement &&
        !['button', 'checkbox', 'radio', 'range'].includes(target.type));

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

  function defaultOutputPath(): string {
    if (!$editor.currentFile) {
      return '';
    }

    return $editor.currentFile.replace(/(\.[^.\\/]+)?$/, '-cutdown.mp4');
  }

  async function chooseOutput(): Promise<void> {
    const settings = await invoke<AppSettings>('get_settings');
    const defaultName = defaultOutputPath().split(/[\\/]/).pop() ?? 'cutdown.mp4';
    const defaultDir = settings.lastExportDir ?? $editor.currentFile?.replace(/[\\/][^\\/]+$/, '') ?? undefined;

    const selected = await save({
      defaultPath: outputPath || (defaultDir ? `${defaultDir}\\${defaultName}` : defaultOutputPath()),
      filters: [{ name: 'MP4 Video', extensions: ['mp4'] }],
    });

    if (selected) {
      outputPath = selected;
      const parent = selected.replace(/[\\/][^\\/]+$/, '');
      if (parent) {
        await invoke('set_last_export_dir', { path: parent });
      }
    }
  }

  async function openExportModal(): Promise<void> {
    outputPath = outputPath || defaultOutputPath();
    exportModalOpen = true;
  }

  async function exportClip(): Promise<void> {
    if (!$editor.currentFile || !outputPath) {
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

    exportProgressPercent = 0;
    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message:
          exportMode === 'range'
            ? 'Exporting I/O range with lossless ffmpeg trim...'
            : 'Running lossless ffmpeg trim...',
      },
    }));

    try {
      const result = await invoke<ExportResult>('export_clip', {
        params: {
          inputPath: $editor.currentFile,
          outputPath,
          audioMode: 'preserve',
          segments: exportSegments,
        },
      });

      exportModalOpen = false;
      exportProgressPercent = 100;
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
    <button type="button" class="tool-button" on:click={chooseClip}>Open</button>
    <button type="button" class="tool-button" disabled={segmentHistory.length === 0} on:click={undoSegmentEdit}>Undo</button>
    <button type="button" class="tool-button" disabled={redoHistory.length === 0} on:click={redoSegmentEdit}>Redo</button>
    <button type="button" class="tool-button" disabled={!canExport} on:click={splitAtCurrentTime}>Split</button>
    <button type="button" class="tool-button" disabled={!$editor.currentFile} on:click={zoomToFit}>Fit</button>
    <div class="toolbar__spacer"></div>
    <span class="toolbar__status">{ffmpegAvailable ? 'Ready' : 'ffmpeg missing'}</span>
    <button type="button" class="tool-button" on:click={() => (settingsModalOpen = true)}>Settings</button>
    <button type="button" class="tool-button" disabled={!canExport} on:click={openExportModal}>Export</button>
  </section>

  <section class="preview-panel">
    <VideoPreview
      bind:this={preview}
      src={$editor.videoSrc}
      currentTime={$editor.currentTime}
      loopEnabled={rangeLoopPlayback}
      loopStart={normalizedRange?.start ?? null}
      loopEnd={normalizedRange?.end ?? null}
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
    <div class="transport-bar__actions">
      <button type="button" class="secondary" disabled={!canUseRange} on:click={splitAtRangeMarkers}>Split I/O</button>
      <button type="button" class="secondary" disabled={!canUseRange} on:click={keepOnlyRange}>Keep range</button>
      <button type="button" class="secondary" disabled={!canUseRange} on:click={deleteOutsideRange}>Trim outside</button>
      <button type="button" class="secondary" disabled={!canUseRange} on:click={zoomToRange}>Zoom range</button>
      <button
        type="button"
        class="secondary"
        class:active={rangeLoopPlayback}
        disabled={!canUseRange}
        on:click={toggleRangeLoop}
      >
        {rangeLoopPlayback ? 'Loop on' : 'Loop range'}
      </button>
      <button type="button" class="secondary" disabled={!selectedSegment || segments.length <= 1} on:click={deleteSelectedSegment}>Delete</button>
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
      <span><kbd>S</kbd> split</span>
      <span><kbd>I</kbd>/<kbd>O</kbd> range</span>
      <span><kbd>L</kbd> loop</span>
      <span><kbd>Z</kbd> zoom range</span>
      <span><kbd>Ctrl</kbd>+wheel zoom</span>
    </div>
    <div class="bottom-bar__status">
      <ProgressBar
        active={$editor.exportStatus.state === 'running'}
        label={$editor.exportStatus.message}
        percent={exportProgressPercent}
      />
      {#if $editor.exportStatus.outputPath}
        <button type="button" class="secondary" on:click={revealExport}>Open Folder</button>
      {/if}
    </div>
  </footer>
</main>

<ExportModal
  open={exportModalOpen}
  {outputPath}
  segmentCount={segments.length}
  duration={outputDuration}
  {rangeDuration}
  canExportRange={canUseRange}
  {exportMode}
  on:close={() => (exportModalOpen = false)}
  on:chooseOutput={chooseOutput}
  on:exportModeChange={(event) => (exportMode = event.detail.mode)}
  on:confirm={exportClip}
/>

<SettingsModal
  visible={settingsModalOpen}
  {watchFolder}
  {watchFolderEnabled}
  {ffmpegStatus}
  on:close={() => (settingsModalOpen = false)}
  on:saved={(event) => {
    watchFolder = event.detail.watchFolder;
    watchFolderEnabled = event.detail.watchFolderEnabled;
  }}
/>
