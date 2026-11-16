// Small measurements shown next to each sent utterance.
//
// These are real numbers from the session, never estimates: how long the
// strokes took and how many words came out.

/** Counts words in finished text. */
export function countWords(text: string): number {
  const words = text.trim().match(/\S+/g);
  return words ? words.length : 0;
}

/**
 * Words per minute for an utterance, or null when the time is too short to
 * mean anything (a single stroke has no measurable duration).
 */
export function wordsPerMinute(words: number, durationMs: number): number | null {
  if (words <= 0 || durationMs < 250) {
    return null;
  }
  return Math.round((words * 60_000) / durationMs);
}

/** Formats a duration as seconds with one decimal, such as `6.2 s`. */
export function formatSeconds(durationMs: number): string {
  return `${(Math.max(0, durationMs) / 1000).toFixed(1)} s`;
}

/** `1 stroke` or `9 strokes`. */
export function plural(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? '' : 's'}`;
}
