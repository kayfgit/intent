import { describe, expect, it } from 'vitest';
import { chordMarks, shapeMarks } from './glyph.js';
import {
  buildKeyCodes,
  DEFAULT_LEFT_THUMB,
  DEFAULT_RIGHT_THUMB,
  keyIndexFor,
  keyLegend,
  LEFT_THUMB,
  RIGHT_THUMB,
} from './keymap.js';
import { DEFAULT_SETTINGS, loadSettings, saveSettings } from './settings.js';
import { countWords, formatSeconds, plural, wordsPerMinute } from './stats.js';

describe('key mapping', () => {
  const codes = buildKeyCodes(DEFAULT_LEFT_THUMB, DEFAULT_RIGHT_THUMB);

  it('puts the fingers on the home row and the thumbs below', () => {
    expect(codes).toEqual([
      'KeyA',
      'KeyS',
      'KeyD',
      'KeyF',
      'KeyC',
      'KeyM',
      'KeyJ',
      'KeyK',
      'KeyL',
      'Semicolon',
    ]);
    expect(keyIndexFor('KeyA', codes)).toBe(0);
    expect(keyIndexFor('KeyC', codes)).toBe(LEFT_THUMB);
    expect(keyIndexFor('KeyM', codes)).toBe(RIGHT_THUMB);
    expect(keyIndexFor('Semicolon', codes)).toBe(9);
  });

  it('ignores keys that play nothing', () => {
    expect(keyIndexFor('KeyQ', codes)).toBeNull();
    expect(keyIndexFor('Space', codes)).toBeNull();
  });

  it('lets the thumbs move to other keys', () => {
    const moved = buildKeyCodes('KeyV', 'KeyN');
    expect(keyIndexFor('KeyV', moved)).toBe(LEFT_THUMB);
    expect(keyIndexFor('KeyN', moved)).toBe(RIGHT_THUMB);
    expect(keyIndexFor('KeyC', moved)).toBeNull();
  });

  it('shows what is printed on the key', () => {
    expect(keyLegend('KeyA')).toBe('A');
    expect(keyLegend('Semicolon')).toBe(';');
    expect(keyLegend('Comma')).toBe(',');
    // On a Brazilian keyboard the key right of L prints a cedilla.
    expect(keyLegend('Semicolon', new Map([['Semicolon', 'ç']]))).toBe('Ç');
    expect(keyLegend('Tab')).toBe('Tab');
  });
});

describe('glyph marks', () => {
  it('reads a chord from its key bits in board order', () => {
    // I-M: left index is bit 3, right middle is bit 7.
    const marks = chordMarks(0b00_1000_1000);
    expect(marks).toHaveLength(10);
    expect(marks.map((mark) => (mark.on ? 'x' : '.')).join('')).toBe('...x...x..');
    expect(marks[3]).toEqual({ hand: 'left', finger: 'I', on: true });
    expect(marks[7]).toEqual({ hand: 'right', finger: 'M', on: true });
  });

  it('draws a left hand pinky to index and a right hand index to pinky', () => {
    const picture = (shape: string, hand: 'left' | 'right', thumb: boolean) =>
      shapeMarks(shape, hand, thumb)
        .map((mark) => (mark.on ? mark.finger : '.'))
        .join('');
    expect(picture('IM', 'left', false)).toBe('..MI');
    expect(picture('IM', 'right', false)).toBe('IM..');
    expect(picture('TI', 'left', true)).toBe('...IT');
    expect(picture('TI', 'right', true)).toBe('TI...');
    expect(picture('IMRP', 'right', true)).toBe('.IMRP');
  });
});

describe('settings', () => {
  function storage(initial: Record<string, string> = {}) {
    const data = new Map(Object.entries(initial));
    return {
      getItem: (key: string) => data.get(key) ?? null,
      setItem: (key: string, value: string) => void data.set(key, value),
    };
  }

  it('starts from defaults', () => {
    expect(loadSettings(storage())).toEqual(DEFAULT_SETTINGS);
    expect(loadSettings(null)).toEqual(DEFAULT_SETTINGS);
    expect(DEFAULT_SETTINGS.providerId).toBe('groq');
  });

  it('round-trips what was saved', () => {
    const store = storage();
    const changed = { ...DEFAULT_SETTINGS, language: 'Portuguese', leftThumb: 'KeyV', model: 'x' };
    expect(saveSettings(store, changed)).toBe(true);
    expect(loadSettings(store)).toEqual(changed);
  });

  it('repairs malformed or partial data', () => {
    const key = 'intent.settings.v1';
    expect(loadSettings(storage({ [key]: 'not json' }))).toEqual(DEFAULT_SETTINGS);
    expect(loadSettings(storage({ [key]: '[1, 2]' }))).toEqual(DEFAULT_SETTINGS);
    const partial = loadSettings(
      storage({ [key]: JSON.stringify({ language: 7, leftThumb: 'KeyQ', apiKey: 'k' }) }),
    );
    expect(partial).toEqual({ ...DEFAULT_SETTINGS, apiKey: 'k' });
  });

  it('survives a browser that blocks storage', () => {
    const blocked = {
      getItem: () => {
        throw new Error('blocked');
      },
      setItem: () => {
        throw new Error('blocked');
      },
    };
    expect(loadSettings(blocked)).toEqual(DEFAULT_SETTINGS);
    expect(saveSettings(blocked, DEFAULT_SETTINGS)).toBe(false);
    expect(saveSettings(null, DEFAULT_SETTINGS)).toBe(false);
  });
});

describe('stats', () => {
  it('counts words', () => {
    expect(countWords('Fix the bug in login.')).toBe(5);
    expect(countWords('  spaced   out  ')).toBe(2);
    expect(countWords('')).toBe(0);
  });

  it('gives words per minute only when the time means something', () => {
    expect(wordsPerMinute(5, 6000)).toBe(50);
    expect(wordsPerMinute(10, 4000)).toBe(150);
    expect(wordsPerMinute(5, 100)).toBeNull();
    expect(wordsPerMinute(0, 6000)).toBeNull();
  });

  it('formats durations and counts', () => {
    expect(formatSeconds(6240)).toBe('6.2 s');
    expect(formatSeconds(-5)).toBe('0.0 s');
    expect(plural(1, 'stroke')).toBe('1 stroke');
    expect(plural(9, 'stroke')).toBe('9 strokes');
  });
});
