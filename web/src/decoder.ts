// Calling a language model to turn an utterance into text.
//
// Every provider here speaks the OpenAI-style chat completions format, so one
// small client covers all of them. The request goes straight from the browser
// to the provider with the user's own key; there is no server in between.

import type { ChatMessage } from './engine.js';
import type { Settings } from './settings.js';

export interface Provider {
  id: string;
  name: string;
  /** Base URL of the OpenAI-compatible API. Empty for the custom provider. */
  baseUrl: string;
  defaultModel: string;
  /** Where to get a key, or null when none is needed. */
  keyUrl: string | null;
  needsKey: boolean;
  /** One plain sentence the user should know before choosing it. */
  note: string;
}

export const PROVIDERS: readonly Provider[] = [
  {
    id: 'groq',
    name: 'Groq',
    baseUrl: 'https://api.groq.com/openai/v1',
    defaultModel: 'openai/gpt-oss-120b',
    keyUrl: 'https://console.groq.com/keys',
    needsKey: true,
    note: 'Free with no card, and very fast. Daily limits apply.',
  },
  {
    id: 'gemini',
    name: 'Google Gemini',
    baseUrl: 'https://generativelanguage.googleapis.com/v1beta/openai',
    defaultModel: 'gemini-3.5-flash-lite',
    keyUrl: 'https://aistudio.google.com/apikey',
    needsKey: true,
    note: 'Free, but Google may use what you send on the free tier to improve its products.',
  },
  {
    id: 'custom',
    name: 'Custom or local',
    baseUrl: '',
    defaultModel: '',
    keyUrl: null,
    needsKey: false,
    note: 'Any OpenAI-compatible server, such as Ollama at http://localhost:11434/v1.',
  },
];

export function providerById(id: string): Provider {
  return PROVIDERS.find((provider) => provider.id === id) ?? (PROVIDERS[0] as Provider);
}

/** The base URL and model a set of settings resolves to. */
export function endpointFor(settings: Settings): { baseUrl: string; model: string } {
  const provider = providerById(settings.providerId);
  const baseUrl = (provider.id === 'custom' ? settings.baseUrl : provider.baseUrl)
    .trim()
    .replace(/\/+$/, '');
  return { baseUrl, model: settings.model.trim() || provider.defaultModel };
}

export type DecodeFailure =
  | 'no-key'
  | 'no-endpoint'
  | 'auth'
  | 'rate-limit'
  | 'network'
  | 'provider'
  | 'empty';

/** A decode that did not produce text, with a message fit to show the user. */
export class DecodeError extends Error {
  readonly kind: DecodeFailure;

  constructor(kind: DecodeFailure, message: string) {
    super(message);
    this.name = 'DecodeError';
    this.kind = kind;
  }
}

/**
 * The chat request. A low temperature keeps the same input giving the same
 * sentence. gpt-oss models think before answering: low effort keeps that
 * short, which saves free daily tokens, and `max_tokens` leaves room for it.
 */
export function requestBody(model: string, messages: readonly ChatMessage[]): object {
  const body: Record<string, unknown> = { model, messages, temperature: 0.2, max_tokens: 1024 };
  if (/gpt-oss/i.test(model)) {
    body.reasoning_effort = 'low';
  }
  return body;
}

type FetchLike = (input: string, init?: RequestInit) => Promise<Response>;

function headersFor(settings: Settings): Record<string, string> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const key = settings.apiKey.trim();
  if (key) {
    headers.Authorization = `Bearer ${key}`;
  }
  return headers;
}

function ready(settings: Settings): { provider: Provider; baseUrl: string; model: string } {
  const provider = providerById(settings.providerId);
  const { baseUrl, model } = endpointFor(settings);
  if (provider.needsKey && !settings.apiKey.trim()) {
    throw new DecodeError('no-key', `Add your ${provider.name} API key in Setup.`);
  }
  if (!baseUrl) {
    throw new DecodeError('no-endpoint', 'Add the server address in Setup.');
  }
  return { provider, baseUrl, model };
}

async function errorMessage(response: Response): Promise<string> {
  try {
    const body = (await response.json()) as { error?: { message?: unknown } | string };
    const detail = typeof body.error === 'string' ? body.error : body.error?.message;
    if (typeof detail === 'string' && detail) {
      return detail;
    }
  } catch {
    // Not JSON. The status line will have to do.
  }
  return `${response.status} ${response.statusText}`.trim();
}

async function request(
  provider: Provider,
  url: string,
  init: RequestInit,
  fetchFn: FetchLike,
): Promise<Response> {
  let response: Response;
  try {
    response = await fetchFn(url, init);
  } catch {
    throw new DecodeError(
      'network',
      `Could not reach ${provider.name}. Check your connection and the address.`,
    );
  }
  if (response.ok) {
    return response;
  }
  if (response.status === 401 || response.status === 403) {
    throw new DecodeError('auth', `${provider.name} rejected the API key. Check it in Setup.`);
  }
  if (response.status === 429) {
    const wait = Number(response.headers.get('retry-after'));
    const when = Number.isFinite(wait) && wait > 0 ? `in ${Math.ceil(wait)} s` : 'in a moment';
    throw new DecodeError(
      'rate-limit',
      `${provider.name}'s free limit was reached. Try again ${when}.`,
    );
  }
  throw new DecodeError(
    'provider',
    `${provider.name} returned an error: ${await errorMessage(response)}`,
  );
}

/**
 * Sends chat messages to the chosen provider and returns the reply text with
 * the time the round trip took.
 */
export async function decode(
  settings: Settings,
  messages: readonly ChatMessage[],
  fetchFn: FetchLike = fetch,
  now: () => number = () => performance.now(),
): Promise<{ text: string; ms: number }> {
  const { provider, baseUrl, model } = ready(settings);
  if (!model) {
    throw new DecodeError('no-endpoint', 'Add a model name in Setup.');
  }
  const started = now();
  const response = await request(
    provider,
    `${baseUrl}/chat/completions`,
    {
      method: 'POST',
      headers: headersFor(settings),
      body: JSON.stringify(requestBody(model, messages)),
    },
    fetchFn,
  );
  let text: unknown;
  try {
    const body = (await response.json()) as { choices?: { message?: { content?: unknown } }[] };
    text = body.choices?.[0]?.message?.content;
  } catch {
    text = undefined;
  }
  if (typeof text !== 'string' || !text.trim()) {
    throw new DecodeError('empty', `${provider.name} answered without any text.`);
  }
  return { text, ms: now() - started };
}

const NOT_CHAT_MODELS = /whisper|tts|orpheus|guard|embed|moderation|image|audio/i;

/** The chat models the provider offers, for the model picker. */
export async function listModels(
  settings: Settings,
  fetchFn: FetchLike = fetch,
): Promise<string[]> {
  const { provider, baseUrl } = ready(settings);
  const response = await request(
    provider,
    `${baseUrl}/models`,
    { headers: headersFor(settings) },
    fetchFn,
  );
  const body = (await response.json()) as { data?: { id?: unknown }[] };
  return (body.data ?? [])
    .map((model) => (typeof model.id === 'string' ? model.id.replace(/^models\//, '') : ''))
    .filter((id) => id && !NOT_CHAT_MODELS.test(id))
    .sort();
}
