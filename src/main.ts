import App from './App.svelte';
import { registerAppIcons } from './lib/registerIcons';
// Bundled so the UI looks the same on every OS (Segoe UI / Cascadia don't exist on Linux).
import '@fontsource-variable/inter/wght.css';
import '@fontsource/jetbrains-mono/400.css';
import '@fontsource/jetbrains-mono/500.css';
import './styles.css';

async function start(): Promise<App> {
  // Plain browser during `npm run dev`: stand in for the Tauri backend.
  if (import.meta.env.DEV && !('__TAURI_INTERNALS__' in window)) {
    await import('./dev/tauriStub');
  }

  registerAppIcons();

  return new App({
    target: document.getElementById('app') as HTMLElement,
  });
}

export default start();
