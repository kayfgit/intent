// Fingering glyphs: the small dot diagrams used across the page.
//
// A filled mark is a pressed key and a hollow one is a lifted key, like a
// woodwind fingering chart. Thumbs are drawn as short bars so the two hands
// read apart at a glance.

export type HandSide = 'left' | 'right';

export interface Mark {
  hand: HandSide;
  /** Finger letter: P, R, M, I or T. */
  finger: string;
  on: boolean;
}

const BOARD: readonly { hand: HandSide; finger: string }[] = [
  { hand: 'left', finger: 'P' },
  { hand: 'left', finger: 'R' },
  { hand: 'left', finger: 'M' },
  { hand: 'left', finger: 'I' },
  { hand: 'left', finger: 'T' },
  { hand: 'right', finger: 'T' },
  { hand: 'right', finger: 'I' },
  { hand: 'right', finger: 'M' },
  { hand: 'right', finger: 'R' },
  { hand: 'right', finger: 'P' },
];

/** The ten marks of a chord, in board order, from its key bits. */
export function chordMarks(bits: number): Mark[] {
  return BOARD.map((key, index) => ({ ...key, on: (bits & (1 << index)) !== 0 }));
}

/**
 * The marks of one hand's shape, in that hand's board order. A left hand
 * reads pinky to index, a right hand index to pinky; the thumb sits on the
 * inside when `withThumb` is set.
 */
export function shapeMarks(shape: string, hand: HandSide, withThumb: boolean): Mark[] {
  const fingers = hand === 'left' ? ['P', 'R', 'M', 'I'] : ['I', 'M', 'R', 'P'];
  const marks = fingers.map((finger) => ({ hand, finger, on: shape.includes(finger) }));
  if (!withThumb) {
    return marks;
  }
  const thumb: Mark = { hand, finger: 'T', on: shape.includes('T') };
  return hand === 'left' ? [...marks, thumb] : [thumb, ...marks];
}

const SVG = 'http://www.w3.org/2000/svg';
const STEP = 10;
const RADIUS = 3.4;
const HAND_GAP = 7;

/** Draws marks as a small inline picture. */
export function glyph(marks: readonly Mark[], label: string): SVGSVGElement {
  const svg = document.createElementNS(SVG, 'svg');
  let x = STEP / 2;
  let previous: HandSide | null = null;
  const shapes: SVGElement[] = [];
  for (const mark of marks) {
    if (previous && previous !== mark.hand) {
      x += HAND_GAP;
    }
    previous = mark.hand;
    let shape: SVGElement;
    if (mark.finger === 'T') {
      shape = document.createElementNS(SVG, 'rect');
      shape.setAttribute('x', String(x - RADIUS));
      shape.setAttribute('y', String(STEP / 2 - 1.8));
      shape.setAttribute('width', String(RADIUS * 2));
      shape.setAttribute('height', '3.6');
      shape.setAttribute('rx', '1.2');
    } else {
      shape = document.createElementNS(SVG, 'circle');
      shape.setAttribute('cx', String(x));
      shape.setAttribute('cy', String(STEP / 2));
      shape.setAttribute('r', String(RADIUS));
    }
    shape.setAttribute('class', `mark mark-${mark.hand}${mark.on ? ' is-on' : ''}`);
    shapes.push(shape);
    x += STEP;
  }
  const width = x - STEP / 2;
  svg.setAttribute('viewBox', `0 0 ${width} ${STEP}`);
  svg.setAttribute('width', String(width * 1.6));
  svg.setAttribute('height', String(STEP * 1.6));
  svg.setAttribute('class', 'glyph');
  svg.setAttribute('role', 'img');
  svg.setAttribute('aria-label', label);
  svg.append(...shapes);
  return svg;
}
