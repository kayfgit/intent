import '@fontsource-variable/recursive/full.css';
import './styles.css';
import layoutToml from '../../layouts/intent-v0.toml?raw';
import { start } from './app.js';
import { Engine, loadEngine } from './engine.js';

async function boot(): Promise<void> {
  try {
    await loadEngine();
    start(Engine.create(layoutToml));
  } catch (error) {
    const box = document.getElementById('boot-error');
    if (box) {
      const reason = error instanceof Error ? error.message : String(error);
      box.textContent = `The engine could not start. ${reason}`;
      box.hidden = false;
    }
    throw error;
  }
}

void boot();
