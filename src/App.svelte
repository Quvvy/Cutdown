<script lang="ts">
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import ExportModal from './components/ExportModal.svelte';
  import ProgressBar from './components/ProgressBar.svelte';
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

  let preview:
    | {
        seekTo(seconds: number): void;
        togglePlayback(): void;
      }
    | undefined;
  let exportModalOpen = false;
  let outputPath = '';
  let segmentHistory: Array<{
    segments: TimelineSegment[];
    selectedSegmentId: string | null;
  }> = [];
  let redoHistory: Array<{
    segments: TimelineSegment[];
    selectedSegmentId: string | null;
  }> = [];

  $: metadata = $editor.metadata;
  $: duration = metadata?.duration ?? 0;
  $: segments = $editor.segments;
  $: selectedSegment = segments.find((segment) => segment.id === $editor.selectedSegmentId) ?? null;
  $: outputDuration = totalSegmentDuration(segments);
  $: fileName = $editor.currentFile?.split(/[\\/]/).pop() ?? 'No file selected';
  $: canExport = Boolean($editor.currentFile && outputDuration > 0);

  function cloneSegments(source: TimelineSegment[]): TimelineSegment[] {
    return source.map((segment) => ({ ...segment }));
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

    segmentHistory = [];
    redoHistory = [];
    editor.update((state) => ({
      ...state,
      currentFile: selected,
      videoSrc: convertFileSrc(selected),
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
      editor.update((state) => ({
        ...state,
        metadata: probed,
        segments: [createFullSegment(probed.duration)],
        selectedSegmentId: null,
        exportStatus: {
          state: 'idle',
          message: `Loaded ${formatBytes(probed.fileSize)} ${probed.codec.toUpperCase()} clip.`,
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

  function splitAtCurrentTime(): void {
    if (!canExport) {
      return;
    }

    const splitTime = $editor.currentTime;
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

  function reorderSegment(id: string, targetIndex: number): void {
    const currentIndex = $editor.segments.findIndex((segment) => segment.id === id);

    if (currentIndex < 0 || currentIndex === targetIndex) {
      return;
    }

    pushUndoSnapshot();
    editor.update((state) => {
      const nextSegments = cloneSegments(state.segments);
      const [movedSegment] = nextSegments.splice(currentIndex, 1);
      nextSegments.splice(targetIndex, 0, movedSegment);

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
  }

  function jumpSegment(direction: -1 | 1): void {
    if (segments.length === 0) {
      return;
    }

    const boundaries = segments.flatMap((segment) => [segment.sourceStart, segment.sourceEnd]).sort((a, b) => a - b);
    const nextBoundary =
      direction > 0
        ? boundaries.find((boundary) => boundary > $editor.currentTime + 0.05)
        : [...boundaries].reverse().find((boundary) => boundary < $editor.currentTime - 0.05);

    if (nextBoundary !== undefined) {
      seekTo(nextBoundary);
    }
  }

  function previewSelectedSegment(): void {
    if (selectedSegment) {
      seekTo(selectedSegment.sourceStart);
    } else if (segments[0]) {
      seekTo(segments[0].sourceStart);
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    if (target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA') {
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

    if (event.key === '[') {
      jumpSegment(-1);
      return;
    }

    if (event.key === ']') {
      jumpSegment(1);
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
    const selected = await save({
      defaultPath: outputPath || defaultOutputPath(),
      filters: [{ name: 'MP4 Video', extensions: ['mp4'] }],
    });

    if (selected) {
      outputPath = selected;
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

    editor.update((state) => ({
      ...state,
      exportStatus: {
        state: 'running',
        message: 'Running lossless ffmpeg trim...',
      },
    }));

    try {
      const result = await invoke<ExportResult>('export_clip', {
        params: {
          inputPath: $editor.currentFile,
          outputPath,
          segments: segments.map((segment) => ({
            start: segment.sourceStart,
            end: segment.sourceEnd,
          })),
        },
      });

      exportModalOpen = false;
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'success',
          message: `Exported ${formatBytes(result.fileSize)} in ${formatTime(result.duration)}.`,
          outputPath: result.outputPath,
          outputSize: result.fileSize,
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
</script>

<svelte:window on:keydown={handleKeydown} />

<main class="shell">
  <header class="top-bar">
    <div>
      <p class="eyebrow">Cutdown MVP</p>
      <h1>{fileName}</h1>
    </div>
    <div class="top-bar__actions">
      <button type="button" class="secondary" on:click={chooseClip}>Open Clip</button>
      <button type="button" disabled={!canExport} on:click={openExportModal}>Export</button>
    </div>
  </header>

  <VideoPreview
    bind:this={preview}
    src={$editor.videoSrc}
    currentTime={$editor.currentTime}
    on:metadata={() => {}}
    on:error={(event) => {
      editor.update((state) => ({
        ...state,
        exportStatus: {
          state: 'error',
          message: event.detail.message,
        },
      }));
    }}
    on:timeupdate={(event) => {
      editor.update((state) => ({ ...state, currentTime: event.detail.currentTime }));
    }}
  />

  <Timeline
    disabled={!$editor.currentFile}
    {duration}
    currentTime={$editor.currentTime}
    {segments}
    selectedSegmentId={$editor.selectedSegmentId}
    on:seek={(event) => seekTo(event.detail.seconds)}
    on:selectSegment={(event) => selectSegment(event.detail.id)}
    on:reorderSegment={(event) => reorderSegment(event.detail.id, event.detail.targetIndex)}
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
      <span>Output</span>
      <strong>{formatTime(outputDuration)}</strong>
    </div>
    <div>
      <span>Segments</span>
      <strong>{segments.length}</strong>
    </div>
    <div class="transport-bar__actions">
      <button type="button" class="secondary" disabled={!canExport} on:click={() => jumpSegment(-1)}>[ Prev</button>
      <button type="button" class="secondary" disabled={!canExport} on:click={() => jumpSegment(1)}>Next ]</button>
      <button type="button" class="secondary" disabled={!canExport} on:click={splitAtCurrentTime}>Split</button>
      <button type="button" class="secondary" disabled={!selectedSegment || segments.length <= 1} on:click={deleteSelectedSegment}>
        Delete Segment
      </button>
      <button type="button" class="secondary" disabled={!canExport} on:click={previewSelectedSegment}>Preview Segment</button>
    </div>
  </section>

  <section class="shortcut-bar" aria-label="Keyboard shortcuts">
    <span><kbd>S</kbd> Split</span>
    <span><kbd>Delete</kbd> Remove segment</span>
    <span><kbd>Ctrl</kbd> + <kbd>Z</kbd> Undo</span>
    <span><kbd>Ctrl</kbd> + <kbd>Y</kbd> Redo</span>
    <span><kbd>[</kbd> / <kbd>]</kbd> Boundaries</span>
    <span>Drag top ruler to scrub</span>
    <span>Drag clips to reorder</span>
  </section>

  <footer class="bottom-bar">
    <div class="metadata">
      {#if metadata}
        <span>{metadata.width}x{metadata.height}</span>
        <span>{metadata.fps.toFixed(2)} fps</span>
        <span>{metadata.codec}</span>
        <span>{formatBytes(metadata.fileSize)}</span>
      {:else}
        <span>Lossless trim preset ready</span>
      {/if}
    </div>
    <ProgressBar active={$editor.exportStatus.state === 'running'} label={$editor.exportStatus.message} />
  </footer>
</main>

<ExportModal
  open={exportModalOpen}
  {outputPath}
  segmentCount={segments.length}
  duration={outputDuration}
  on:close={() => (exportModalOpen = false)}
  on:chooseOutput={chooseOutput}
  on:confirm={exportClip}
/>
