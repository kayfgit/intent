// The typed doorway to the Rust engine.
//
// The engine is compiled to WebAssembly from crates/intent-wasm and speaks
// JSON. The interfaces here mirror the Rust view types in
// crates/intent-wasm/src/view.rs field for field; a Rust test locks that shape.

import init, { cleanReply, IntentSession, initSync } from './wasm/intent.js';

export type Kind = 'concept' | 'text' | 'command';
export type PlaneName = 'core' | 'extended' | 'literal' | 'user';

/** A chord and what it does. */
export interface EntryView {
  chord: string;
  /** Key bits: bit 0 is the left pinky, bit 9 the right pinky. */
  bits: number;
  kind: Kind;
  label: string;
  gloss: string | null;
}

export interface MemberView extends EntryView {
  /** The right-hand finger shape that selects this member. */
  right: string;
}

export interface FamilyView {
  id: string;
  name: string;
  plane: PlaneName;
  kind: Kind;
  /** The left-hand finger shape that selects this family. */
  left: string;
  members: MemberView[];
}

export interface SpellView {
  shape: string;
  text: string;
  shifted: string | null;
}

export interface LayoutView {
  name: string;
  version: string;
  description: string;
  finger_shapes: string[];
  spell_shapes: string[];
  thumbs: EntryView[];
  spelling: SpellView[];
  families: FamilyView[];
}

export interface PendingView {
  bits: number;
  chord: string;
  left: string;
  right: string;
  left_thumb: boolean;
  right_thumb: boolean;
  family_plane: PlaneName;
  preview: EntryView | null;
}

export interface ItemView {
  kind: 'concept' | 'word';
  text: string;
}

export interface BufferView {
  items: ItemView[];
  word_open: boolean;
  wire: string;
}

export type EffectView =
  | { type: 'updated' }
  | { type: 'ignored' }
  | { type: 'send'; wire: string; items: ItemView[] }
  | { type: 'key'; key: string }
  | { type: 'unmapped' };

export interface OutcomeView {
  chord: string;
  bits: number;
  started_ms: number;
  ended_ms: number;
  action: EntryView | null;
  effect: EffectView;
}

export interface UpdateView {
  held: number;
  pending: PendingView;
  outcome: OutcomeView | null;
  buffer: BufferView;
}

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

/** An earlier exchange: what was sent and the text it became. */
export interface Turn {
  utterance: string;
  text: string;
}

/** Loads the WebAssembly module in a browser. Call once before `Engine.create`. */
export async function loadEngine(): Promise<void> {
  await init();
}

/** Loads the WebAssembly module from bytes, for tests that run outside a browser. */
export function loadEngineFromBytes(bytes: BufferSource): void {
  initSync({ module: bytes });
}

/** One chord input session. */
export class Engine {
  private constructor(private readonly session: IntentSession) {}

  /** Compiles a layout and starts a session. Throws if the layout has errors. */
  static create(layoutToml: string): Engine {
    return new Engine(new IntentSession(layoutToml));
  }

  layout(): LayoutView {
    return JSON.parse(this.session.layout()) as LayoutView;
  }

  /** A key went down. `key` is 0 (left pinky) to 9 (right pinky). */
  press(key: number, atMs: number): UpdateView {
    return JSON.parse(this.session.press(key, atMs)) as UpdateView;
  }

  release(key: number, atMs: number): UpdateView {
    return JSON.parse(this.session.release(key, atMs)) as UpdateView;
  }

  /** Applies a chord written in notation, such as `I-M`. */
  strike(chord: string, atMs: number): UpdateView {
    return JSON.parse(this.session.strike(chord, atMs)) as UpdateView;
  }

  /** Drops any stroke in progress. */
  releaseAll(): UpdateView {
    return JSON.parse(this.session.releaseAll()) as UpdateView;
  }

  state(): UpdateView {
    return JSON.parse(this.session.state()) as UpdateView;
  }

  /** Chat messages asking a model to decode the last sent utterance. */
  decodeMessages(history: readonly Turn[], language: string): ChatMessage[] {
    return JSON.parse(
      this.session.decodeMessages(JSON.stringify(history), language),
    ) as ChatMessage[];
  }
}

export { cleanReply };
