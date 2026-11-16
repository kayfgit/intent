// What the demo remembers between visits, kept in the browser only.

import {
  DEFAULT_LEFT_THUMB,
  DEFAULT_RIGHT_THUMB,
  LEFT_THUMB_CHOICES,
  RIGHT_THUMB_CHOICES,
} from './keymap.js';

export interface Settings {
  /** Which decoder provider to call. */
  providerId: string;
  /** Base URL, used only by the custom provider. */
  baseUrl: string;
  /** The user's own API key. It is sent to the chosen provider and nowhere else. */
  apiKey: string;
  /** Model name. Empty means the provider's default. */
  model: string;
  /** The language the decoder writes in. */
  language: string;
  leftThumb: string;
  rightThumb: string;
}

export const DEFAULT_SETTINGS: Settings = {
  providerId: 'groq',
  baseUrl: '',
  apiKey: '',
  model: '',
  language: 'English',
  leftThumb: DEFAULT_LEFT_THUMB,
  rightThumb: DEFAULT_RIGHT_THUMB,
};

const STORAGE_KEY = 'intent.settings.v1';

type ReadStorage = Pick<Storage, 'getItem'>;
type WriteStorage = Pick<Storage, 'setItem'>;

function text(value: unknown, fallback: string): string {
  return typeof value === 'string' ? value : fallback;
}

function oneOf(value: unknown, choices: readonly string[], fallback: string): string {
  return typeof value === 'string' && choices.includes(value) ? value : fallback;
}

/** Reads settings, falling back to defaults for anything missing or malformed. */
export function loadSettings(storage: ReadStorage | null): Settings {
  let stored: Record<string, unknown> = {};
  try {
    const raw = storage?.getItem(STORAGE_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : null;
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      stored = parsed as Record<string, unknown>;
    }
  } catch {
    // Private windows and blocked storage throw. Defaults are fine.
  }
  const defaults = DEFAULT_SETTINGS;
  return {
    providerId: text(stored.providerId, defaults.providerId),
    baseUrl: text(stored.baseUrl, defaults.baseUrl),
    apiKey: text(stored.apiKey, defaults.apiKey),
    model: text(stored.model, defaults.model),
    language: text(stored.language, defaults.language),
    leftThumb: oneOf(stored.leftThumb, LEFT_THUMB_CHOICES, defaults.leftThumb),
    rightThumb: oneOf(stored.rightThumb, RIGHT_THUMB_CHOICES, defaults.rightThumb),
  };
}

/** Saves settings. Returns false when the browser refuses to store them. */
export function saveSettings(storage: WriteStorage | null, settings: Settings): boolean {
  try {
    if (!storage) {
      return false;
    }
    storage.setItem(STORAGE_KEY, JSON.stringify(settings));
    return true;
  } catch {
    return false;
  }
}
