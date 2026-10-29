import { rmSync } from 'node:fs';
import { resolve } from 'node:path';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig, type Plugin } from 'vite';
import { devTauriBackend } from './dev/viteDevBackend';

/**
 * public/ffmpeg holds optional dev binaries that the Rust side reads from disk. Vite copies
 * public/ into dist/, and Tauri embeds dist/ in the app, so drop them from the build output.
 */
function stripDevFfmpeg(): Plugin {
  let outDir = 'dist';
  return {
    name: 'cutdown-strip-dev-ffmpeg',
    apply: 'build',
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir);
    },
    closeBundle() {
      rmSync(resolve(outDir, 'ffmpeg'), { recursive: true, force: true });
    },
  };
}

export default defineConfig({
  plugins: [svelte(), devTauriBackend(), stripDevFfmpeg()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});
