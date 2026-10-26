import App from './App.svelte';
import { registerAppIcons } from './lib/registerIcons';
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
