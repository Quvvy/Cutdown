import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vite';
import { devTauriBackend } from './dev/viteDevBackend';

export default defineConfig({
  plugins: [svelte(), devTauriBackend()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
});
