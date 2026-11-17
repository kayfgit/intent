// Which physical keys play the ten Intent keys.
//
// Keys are matched by `KeyboardEvent.code`, the physical position, so the
// mapping is the same on QWERTY, AZERTY, ABNT2 or anything else.

/** Board positions of the two thumbs. */
export const LEFT_THUMB = 4;
export const RIGHT_THUMB = 5;

export const LEFT_THUMB_CHOICES = ['KeyC', 'KeyV', 'KeyB'] as const;
export const RIGHT_THUMB_CHOICES = ['KeyM', 'KeyN', 'Comma'] as const;

export const DEFAULT_LEFT_THUMB = 'KeyC';
export const DEFAULT_RIGHT_THUMB = 'KeyM';

/** Finger names by board position, for labels and screen readers. */
export const FINGER_NAMES = [
  'left pinky',
  'left ring',
  'left middle',
  'left index',
  'left thumb',
  'right thumb',
  'right index',
  'right middle',
  'right ring',
  'right pinky',
] as const;

/** Finger letters by board position, as used in chord notation. */
export const FINGER_LETTERS = ['P', 'R', 'M', 'I', 'T', 'T', 'I', 'M', 'R', 'P'] as const;

/**
 * The ten key codes in board order: left pinky to left thumb, then right
 * thumb to right pinky. The fingers rest on the home row.
 */
export function buildKeyCodes(leftThumb: string, rightThumb: string): string[] {
  return [
    'KeyA',
    'KeyS',
    'KeyD',
    'KeyF',
    leftThumb,
    rightThumb,
    'KeyJ',
    'KeyK',
    'KeyL',
    'Semicolon',
  ];
}

/** The board position a physical key plays, or null if it plays none. */
export function keyIndexFor(code: string, codes: readonly string[]): number | null {
  const index = codes.indexOf(code);
  return index === -1 ? null : index;
}

const US_LEGENDS: Record<string, string> = {
  Semicolon: ';',
  Comma: ',',
};

/**
 * What is printed on a key. `legends` comes from the browser when it can
 * report the real keyboard layout; otherwise the US legend is used.
 */
export function keyLegend(code: string, legends?: ReadonlyMap<string, string>): string {
  const known = legends?.get(code);
  if (known) {
    return known.toUpperCase();
  }
  const us = US_LEGENDS[code];
  if (us) {
    return us;
  }
  return code.startsWith('Key') ? code.slice(3) : code;
}
