// Runs the real WebAssembly engine, so these tests need `pnpm wasm` first.

import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
import layoutToml from '../../layouts/intent-v0.toml?raw';
import { cleanReply, Engine, loadEngineFromBytes } from './engine.js';

// Board positions.
const LEFT_INDEX = 3;
const LEFT_THUMB = 4;
const RIGHT_THUMB = 5;
const RIGHT_MIDDLE = 7;

beforeAll(() => {
  loadEngineFromBytes(readFileSync(new URL('./wasm/intent_bg.wasm', import.meta.url)));
});

function stroke(engine: Engine, keys: number[], at: number) {
  for (const key of keys) {
    engine.press(key, at);
  }
  let update = engine.state();
  for (const key of keys) {
    update = engine.release(key, at + 40);
  }
  return update;
}

describe('the engine in WebAssembly', () => {
  it('describes the shipped layout', () => {
    const layout = Engine.create(layoutToml).layout();
    expect(layout.name).toBe('intent-v0');
    expect(layout.finger_shapes).toHaveLength(15);
    expect(layout.spelling).toHaveLength(30);
    expect(layout.thumbs.map((thumb) => thumb.label)).toEqual(['undo', 'space', 'send']);
    const act = layout.families.find((family) => family.id === 'act');
    expect(act?.left).toBe('I');
    expect(act?.members[1]).toMatchObject({ right: 'M', label: 'FIX', chord: 'I-M' });
  });

  it('previews a chord while keys are held', () => {
    const engine = Engine.create(layoutToml);
    engine.press(LEFT_INDEX, 0);
    const held = engine.press(RIGHT_MIDDLE, 10);
    expect(held.pending.chord).toBe('I-M');
    expect(held.pending.left).toBe('I');
    expect(held.pending.right).toBe('M');
    expect(held.pending.family_plane).toBe('core');
    expect(held.pending.preview?.label).toBe('FIX');
    expect(held.held).toBe((1 << LEFT_INDEX) | (1 << RIGHT_MIDDLE));
  });

  it('builds a buffer and sends it', () => {
    const engine = Engine.create(layoutToml);
    const fixed = stroke(engine, [LEFT_INDEX, RIGHT_MIDDLE], 0);
    expect(fixed.outcome?.effect).toEqual({ type: 'updated' });
    expect(fixed.buffer.wire).toBe('FIX');

    const sent = stroke(engine, [LEFT_THUMB, RIGHT_THUMB], 500);
    expect(sent.outcome?.effect).toEqual({
      type: 'send',
      wire: 'FIX',
      items: [{ kind: 'concept', text: 'FIX' }],
    });
    expect(sent.buffer.items).toEqual([]);

    const messages = engine.decodeMessages(
      [{ utterance: 'ADD TEST', text: 'Add a test.' }],
      'English',
    );
    expect(messages[0]?.role).toBe('system');
    expect(messages.at(-1)).toEqual({ role: 'user', content: 'FIX' });
    expect(messages.at(-2)).toEqual({ role: 'assistant', content: 'Add a test.' });
  });

  it('strikes chords by name and drops strokes on release-all', () => {
    const engine = Engine.create(layoutToml);
    expect(engine.strike('I-I', 0).buffer.wire).toBe('MAKE');
    engine.press(LEFT_INDEX, 10);
    expect(engine.releaseAll().pending.chord).toBe('');
    expect(engine.state().buffer.wire).toBe('MAKE');
  });

  it('throws readable errors', () => {
    expect(() =>
      Engine.create('[layout]\nname = "x"\nversion = "1"\n[thumbs]\nleft = "fly"\n'),
    ).toThrow(/thumbs\.left: unknown command "fly"/);
    const engine = Engine.create(layoutToml);
    expect(() => engine.press(12, 0)).toThrow(/key 12 does not exist/);
    expect(() => engine.strike('nope', 0)).toThrow(/is not a chord/);
    expect(() => engine.decodeMessages([], 'English')).toThrow(/nothing has been sent yet/);
  });

  it('tidies model replies', () => {
    expect(cleanReply(' "Fix the bug." ')).toBe('Fix the bug.');
  });
});
