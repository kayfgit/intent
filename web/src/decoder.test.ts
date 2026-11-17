import { describe, expect, it } from 'vitest';
import {
  DecodeError,
  decode,
  endpointFor,
  listModels,
  PROVIDERS,
  providerById,
} from './decoder.js';
import type { ChatMessage } from './engine.js';
import { DEFAULT_SETTINGS, type Settings } from './settings.js';

const MESSAGES: ChatMessage[] = [
  { role: 'system', content: 'Expand.' },
  { role: 'user', content: 'FIX BUG' },
];

const WITH_KEY: Settings = { ...DEFAULT_SETTINGS, apiKey: ' secret ' };

interface Call {
  url: string;
  init: RequestInit | undefined;
}

/** A fetch stand-in that records calls and replies with a canned response. */
function fakeFetch(reply: () => Response | Promise<Response>) {
  const calls: Call[] = [];
  const fetchFn = async (url: string, init?: RequestInit) => {
    calls.push({ url, init });
    return reply();
  };
  return { calls, fetchFn };
}

function json(body: unknown, status = 200, headers: Record<string, string> = {}) {
  return new Response(JSON.stringify(body), { status, headers });
}

async function failure(run: Promise<unknown>): Promise<DecodeError> {
  try {
    await run;
  } catch (error) {
    expect(error).toBeInstanceOf(DecodeError);
    return error as DecodeError;
  }
  throw new Error('expected the call to fail');
}

describe('providers', () => {
  it('defaults to Groq, the researched free pick', () => {
    expect(PROVIDERS[0]?.id).toBe('groq');
    expect(providerById('nope').id).toBe('groq');
    expect(endpointFor(DEFAULT_SETTINGS)).toEqual({
      baseUrl: 'https://api.groq.com/openai/v1',
      model: 'openai/gpt-oss-120b',
    });
  });

  it('lets the model be overridden and the custom server be set', () => {
    expect(endpointFor({ ...DEFAULT_SETTINGS, model: ' other ' }).model).toBe('other');
    expect(
      endpointFor({
        ...DEFAULT_SETTINGS,
        providerId: 'custom',
        baseUrl: 'http://localhost:11434/v1/ ',
        model: 'llama3',
      }),
    ).toEqual({ baseUrl: 'http://localhost:11434/v1', model: 'llama3' });
  });
});

describe('decode', () => {
  it('posts the messages and returns the reply with its timing', async () => {
    const { calls, fetchFn } = fakeFetch(() =>
      json({ choices: [{ message: { content: 'Fix the bug.' } }] }),
    );
    const times = [100, 460];
    const result = await decode(WITH_KEY, MESSAGES, fetchFn, () => times.shift() ?? 0);

    expect(result).toEqual({ text: 'Fix the bug.', ms: 360 });
    expect(calls).toHaveLength(1);
    expect(calls[0]?.url).toBe('https://api.groq.com/openai/v1/chat/completions');
    expect(calls[0]?.init?.method).toBe('POST');
    expect(calls[0]?.init?.headers).toEqual({
      'Content-Type': 'application/json',
      Authorization: 'Bearer secret',
    });
    expect(JSON.parse(String(calls[0]?.init?.body))).toEqual({
      model: 'openai/gpt-oss-120b',
      messages: MESSAGES,
      temperature: 0.2,
      max_tokens: 1024,
      reasoning_effort: 'low',
    });
  });

  it('asks for a key before calling a provider that needs one', async () => {
    const { calls, fetchFn } = fakeFetch(() => json({}));
    const error = await failure(decode(DEFAULT_SETTINGS, MESSAGES, fetchFn));
    expect(error.kind).toBe('no-key');
    expect(error.message).toBe('Add your Groq API key in Setup.');
    expect(calls).toHaveLength(0);
  });

  it('calls a local server without a key', async () => {
    const { calls, fetchFn } = fakeFetch(() => json({ choices: [{ message: { content: 'ok' } }] }));
    const local: Settings = {
      ...DEFAULT_SETTINGS,
      providerId: 'custom',
      baseUrl: 'http://localhost:11434/v1',
      model: 'llama3',
    };
    await decode(local, MESSAGES, fetchFn);
    expect(calls[0]?.url).toBe('http://localhost:11434/v1/chat/completions');
    // Only gpt-oss models get a reasoning effort; other servers may reject the field.
    expect(JSON.parse(String(calls[0]?.init?.body))).not.toHaveProperty('reasoning_effort');
    expect(calls[0]?.init?.headers).toEqual({ 'Content-Type': 'application/json' });
  });

  it('needs an address and a model for a custom server', async () => {
    const { fetchFn } = fakeFetch(() => json({}));
    const custom: Settings = { ...DEFAULT_SETTINGS, providerId: 'custom' };
    expect((await failure(decode(custom, MESSAGES, fetchFn))).message).toBe(
      'Add the server address in Setup.',
    );
    const noModel = { ...custom, baseUrl: 'http://localhost:1/v1' };
    expect((await failure(decode(noModel, MESSAGES, fetchFn))).message).toBe(
      'Add a model name in Setup.',
    );
  });

  it('explains a rejected key', async () => {
    const { fetchFn } = fakeFetch(() => json({ error: { message: 'Invalid API Key' } }, 401));
    const error = await failure(decode(WITH_KEY, MESSAGES, fetchFn));
    expect(error.kind).toBe('auth');
    expect(error.message).toBe('Groq rejected the API key. Check it in Setup.');
  });

  it('says when to retry after hitting the free limit', async () => {
    const limited = fakeFetch(() => json({}, 429, { 'retry-after': '7.2' }));
    const error = await failure(decode(WITH_KEY, MESSAGES, limited.fetchFn));
    expect(error.kind).toBe('rate-limit');
    expect(error.message).toBe("Groq's free limit was reached. Try again in 8 s.");

    const unknown = fakeFetch(() => json({}, 429));
    expect((await failure(decode(WITH_KEY, MESSAGES, unknown.fetchFn))).message).toBe(
      "Groq's free limit was reached. Try again in a moment.",
    );
  });

  it("passes on the provider's own error message", async () => {
    const detailed = fakeFetch(() => json({ error: { message: 'model not found' } }, 404));
    expect((await failure(decode(WITH_KEY, MESSAGES, detailed.fetchFn))).message).toBe(
      'Groq returned an error: model not found',
    );
    const plain = fakeFetch(() => json({ error: 'overloaded' }, 503));
    expect((await failure(decode(WITH_KEY, MESSAGES, plain.fetchFn))).message).toBe(
      'Groq returned an error: overloaded',
    );
    const opaque = fakeFetch(
      () => new Response('<html>', { status: 502, statusText: 'Bad Gateway' }),
    );
    const error = await failure(decode(WITH_KEY, MESSAGES, opaque.fetchFn));
    expect(error.kind).toBe('provider');
    expect(error.message).toBe('Groq returned an error: 502 Bad Gateway');
  });

  it('reports a network failure plainly', async () => {
    const fetchFn = async () => {
      throw new TypeError('Failed to fetch');
    };
    const error = await failure(decode(WITH_KEY, MESSAGES, fetchFn));
    expect(error.kind).toBe('network');
    expect(error.message).toBe('Could not reach Groq. Check your connection and the address.');
  });

  it('treats a reply without text as a failure', async () => {
    for (const body of [{}, { choices: [] }, { choices: [{ message: { content: '  ' } }] }]) {
      const { fetchFn } = fakeFetch(() => json(body));
      expect((await failure(decode(WITH_KEY, MESSAGES, fetchFn))).kind).toBe('empty');
    }
    const notJson = fakeFetch(() => new Response('oops'));
    expect((await failure(decode(WITH_KEY, MESSAGES, notJson.fetchFn))).kind).toBe('empty');
  });
});

describe('listModels', () => {
  it('lists chat models only, sorted', async () => {
    const { calls, fetchFn } = fakeFetch(() =>
      json({
        data: [
          { id: 'whisper-large-v3' },
          { id: 'llama-3.3-70b-versatile' },
          { id: 'meta-llama/llama-prompt-guard-2-22m' },
          { id: 'models/gemini-3.5-flash-lite' },
          { id: 42 },
          { id: 'canopylabs/orpheus-v1-english' },
          { id: 'llama-3.1-8b-instant' },
        ],
      }),
    );
    expect(await listModels(WITH_KEY, fetchFn)).toEqual([
      'gemini-3.5-flash-lite',
      'llama-3.1-8b-instant',
      'llama-3.3-70b-versatile',
    ]);
    expect(calls[0]?.url).toBe('https://api.groq.com/openai/v1/models');
  });

  it('copes with an empty reply and still needs a key', async () => {
    const { fetchFn } = fakeFetch(() => json({}));
    expect(await listModels(WITH_KEY, fetchFn)).toEqual([]);
    expect((await failure(listModels(DEFAULT_SETTINGS, fetchFn))).kind).toBe('no-key');
  });
});
