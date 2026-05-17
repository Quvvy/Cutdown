<script lang="ts">
  import { convertFileSrc, invoke } from '@tauri-apps/api/core';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import ExportModal from './components/ExportModal.svelte';
  import ProgressBar from './components/ProgressBar.svelte';
  import Timeline from './components/Timeline.svelte';
  import VideoPreview from './components/VideoPreview.svelte';
  import { clamp, formatBytes, formatTime } from './lib/format';
  import { editor, type VideoMetadata } from './stores/editor';

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

  $: metadata = $editor.metadata;
  $: duration = metadata?.duration ?? 0;
  $: outPoint = $editor.outPoint || duration;
  $: trimDuration = Math.max(0, outPoint - $editor.inPoint);
  $: fileName = $editor.currentFile?.split(/[\\/]/).pop() ?? 'No file selected';
  $: canExport = Boolean($editor.currentFile && trimDuration > 0);

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

    editor.update((state) => ({
      ...state,
      currentFile: selected,
      videoSrc: convertFileSrc(selected),
      metadata: null,
      currentTime: 0,
      inPoint: 0,
      outPoint: 0,
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
        outPoint: probed.duration,
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

  function setInPoint(seconds: number): void {
    editor.update((state) => ({
      ...state,
      inPoint: clamp(seconds, 0, Math.max(0, outPoint - 0.1)),
    }));
  }

  function setOutPoint(seconds: number): void {
    editor.update((state) => ({
      ...state,
      outPoint: clamp(seconds, Math.min(state.inPoint + 0.1, duration), duration),
    }));
  }

  function handleKeydown(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    if (!canExport || target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA') {
      return;
    }

    if (event.code === 'Space') {
      event.preventDefault();
      preview?.togglePlayback();
      return;
    }

    if (event.key.toLowerCase() === 'i') {
      setInPoint($editor.currentTime);
      return;
    }

    if (event.key.toLowerCase() === 'o') {
      setOutPoint($editor.currentTime);
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
          inPoint: $editor.inPoint,
          outPoint,
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
    on:metadata={(event) => {
      if (!duration && event.detail.duration) {
        setOutPoint(event.detail.duration);
      }
    }}
    on:timeupdate={(event) => {
      editor.update((state) => ({ ...state, currentTime: event.detail.currentTime }));
    }}
  />

  <Timeline
    disabled={!$editor.currentFile}
    {duration}
    currentTime={$editor.currentTime}
    inPoint={$editor.inPoint}
    {outPoint}
    on:seek={(event) => seekTo(event.detail.seconds)}
    on:setIn={(event) => setInPoint(event.detail.seconds)}
    on:setOut={(event) => setOutPoint(event.detail.seconds)}
  />

  <section class="trim-bar" aria-label="Trim controls">
    <div>
      <span>In</span>
      <strong>{formatTime($editor.inPoint)}</strong>
    </div>
    <div>
      <span>Out</span>
      <strong>{formatTime(outPoint)}</strong>
    </div>
    <div>
      <span>Duration</span>
      <strong>{formatTime(trimDuration)}</strong>
    </div>
    <button type="button" class="secondary" disabled={!canExport} on:click={() => seekTo($editor.inPoint)}>
      Preview Trim
    </button>
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
  inPoint={$editor.inPoint}
  {outPoint}
  on:close={() => (exportModalOpen = false)}
  on:chooseOutput={chooseOutput}
  on:confirm={exportClip}
/>
