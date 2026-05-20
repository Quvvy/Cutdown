import App from './App.svelte';
import { registerAppIcons } from './lib/registerIcons';
import './styles.css';

registerAppIcons();

const app = new App({
  target: document.getElementById('app') as HTMLElement,
});

export default app;
