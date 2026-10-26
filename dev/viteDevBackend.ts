// Dev-server endpoints that stand in for the Rust backend when the editor runs in a
// plain browser (see src/dev/tauriStub.ts). Serve only; never part of a build.
import { spawn } from 'node:child_process';
import { createReadStream, existsSync, statSync } from 'node:fs';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { extname, join } from 'node:path';
import type { Plugin } from 'vite';

const MIME: Record<string, string> = {
  '.mp4': 'video/mp4',
  '.m4v': 'video/mp4',
  '.mov': 'video/quicktime',
  '.webm': 'video/webm',
  '.mkv': 'video/x-matroska',
};

function findTool(name: 'ffmpeg' | 'ffprobe'): string {
  const bundled = join(process.cwd(), 'public', 'ffmpeg', `${name}.exe`);
  return existsSync(bundled) ? bundled : name;
}

function run(tool: string, args: string[]): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    const child = spawn(tool, args, { windowsHide: true });
    const out: Buffer[] = [];
    const err: Buffer[] = [];
    child.stdout.on('data', (chunk: Buffer) => out.push(chunk));
    child.stderr.on('data', (chunk: Buffer) => err.push(chunk));
    child.on('error', reject);
    child.on('close', (code) =>
      code === 0 ? resolve(Buffer.concat(out)) : reject(new Error(Buffer.concat(err).toString().trim() || `${tool} exited ${code}`)),
    );
  });
}

function parseRate(rate: string | undefined): number {
  const [num, den] = (rate ?? '0/1').split('/').map(Number);
  return den ? num / den : 0;
}

async function probe(path: string) {
  const raw = await run(findTool('ffprobe'), ['-v', 'error', '-show_streams', '-show_format', '-of', 'json', path]);
  const info = JSON.parse(raw.toString());
  const video = info.streams.find((stream: { codec_type: string }) => stream.codec_type === 'video');
  const audio = info.streams.find((stream: { codec_type: string }) => stream.codec_type === 'audio');
  if (!video) {
    throw new Error('No video stream found.');
  }
  // Same as upright_dimensions in Rust: portrait phone video is landscape frames + a 90° flag.
  const rotation = Number(
    video.side_data_list?.find((item: { rotation?: number }) => item.rotation !== undefined)?.rotation ?? video.tags?.rotate ?? 0,
  );
  const sideways = ((Math.round(rotation) % 180) + 180) % 180 === 90;
  return {
    duration: Number(info.format.duration),
    fps: parseRate(video.avg_frame_rate) || parseRate(video.r_frame_rate),
    maxFps: parseRate(video.r_frame_rate) || parseRate(video.avg_frame_rate),
    variableFrameRate: (() => {
      const average = parseRate(video.avg_frame_rate);
      const max = parseRate(video.r_frame_rate);
      return average > 0 && max > 0 && Math.abs(max - average) / max > 0.05;
    })(),
    codec: video.codec_name,
    width: sideways ? video.height : video.width,
    height: sideways ? video.width : video.height,
    fileSize: Number(info.format.size),
    audioCodec: audio?.codec_name ?? null,
    audioChannels: audio?.channels ?? null,
    pixelFormat: video.pix_fmt ?? null,
    // Same as probe_video in Rust: container is the file extension.
    container: extname(path).slice(1).toLowerCase(),
  };
}

// Peak amplitude per bucket, 0..1, like extract_waveform in src-tauri/src/ffmpeg.rs.
async function waveform(path: string, buckets: number): Promise<number[]> {
  const pcm = await run(findTool('ffmpeg'), ['-v', 'error', '-i', path, '-vn', '-ac', '1', '-ar', '8000', '-f', 's16le', '-']);
  const samples = pcm.length >> 1;
  if (!samples) {
    return [];
  }
  const count = Math.min(8000, Math.max(64, buckets));
  const perBucket = Math.max(1, samples / count);
  const peaks: number[] = [];
  for (let bucket = 0; bucket < count; bucket++) {
    let peak = 0;
    const end = Math.min(samples, Math.floor((bucket + 1) * perBucket));
    for (let i = Math.floor(bucket * perBucket); i < end; i++) {
      peak = Math.max(peak, Math.abs(pcm.readInt16LE(i * 2)));
    }
    peaks.push(peak / 32768);
  }
  return peaks;
}

// Same ffmpeg arguments as extract_thumbnail in src-tauri/src/ffmpeg.rs.
async function thumbnail(path: string, time: number, height: number): Promise<string | null> {
  const seek = Number.isFinite(time) ? Math.max(0, time) : 0;
  const h = Math.min(240, Math.max(24, height || 72));
  const jpeg = await run(findTool('ffmpeg'), [
    '-hide_banner', '-loglevel', 'error', '-ss', seek.toFixed(3), '-i', path,
    '-frames:v', '1', '-an', '-vf', `scale=-2:${h}`, '-q:v', '6', '-f', 'image2pipe', '-c:v', 'mjpeg', 'pipe:1',
  ]);
  return jpeg.length ? `data:image/jpeg;base64,${jpeg.toString('base64')}` : null;
}

// Range-aware file streaming so the <video> element can seek.
function serveFile(path: string, req: IncomingMessage, res: ServerResponse): void {
  const size = statSync(path).size;
  const type = MIME[extname(path).toLowerCase()] ?? 'application/octet-stream';
  const range = /bytes=(\d*)-(\d*)/.exec(req.headers.range ?? '');
  if (!range) {
    res.writeHead(200, { 'Content-Type': type, 'Content-Length': size, 'Accept-Ranges': 'bytes' });
    createReadStream(path).pipe(res);
    return;
  }
  const start = range[1] ? Number(range[1]) : size - Number(range[2]);
  const end = range[1] && range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
  res.writeHead(206, {
    'Content-Type': type,
    'Content-Length': end - start + 1,
    'Content-Range': `bytes ${start}-${end}/${size}`,
    'Accept-Ranges': 'bytes',
  });
  createReadStream(path, { start, end }).pipe(res);
}

function sendJson(res: ServerResponse, status: number, body: unknown): void {
  res.writeHead(status, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(body));
}

export function devTauriBackend(): Plugin {
  return {
    name: 'cutdown-dev-tauri-backend',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use('/__dev', (req, res) => {
        const url = new URL(req.url ?? '/', 'http://localhost');
        // /file/<encoded path> (video src) or ?path=<path> (everything else)
        const isFile = url.pathname.startsWith('/file/');
        const path = isFile ? decodeURIComponent(url.pathname.slice('/file/'.length)) : url.searchParams.get('path') ?? '';
        const handle = async () => {
          if (url.pathname === '/exists') {
            return sendJson(res, 200, existsSync(path));
          }
          // Only hand out media files, not arbitrary files from disk.
          if (!MIME[extname(path).toLowerCase()]) {
            return sendJson(res, 403, { error: `Dev backend only serves video files: ${path}` });
          }
          if (!existsSync(path)) {
            return sendJson(res, 404, { error: `File not found: ${path}` });
          }
          if (isFile) {
            return serveFile(path, req, res);
          }
          if (url.pathname === '/probe') {
            return sendJson(res, 200, await probe(path));
          }
          if (url.pathname === '/thumbnail') {
            const time = Number(url.searchParams.get('time'));
            return sendJson(res, 200, await thumbnail(path, time, Number(url.searchParams.get('height'))));
          }
          if (url.pathname === '/waveform') {
            return sendJson(res, 200, await waveform(path, Number(url.searchParams.get('buckets')) || 2400));
          }
          sendJson(res, 404, { error: `Unknown dev endpoint ${url.pathname}` });
        };
        handle().catch((error: Error) => sendJson(res, 500, { error: error.message }));
      });
    },
  };
}
