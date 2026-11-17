// The page: draws the engine's state and feeds it key presses.

import { DecodeError, decode, listModels, PROVIDERS, providerById } from './decoder.js';
import {
  cleanReply,
  type Engine,
  type EntryView,
  type FamilyView,
  type OutcomeView,
  type PlaneName,
  type Turn,
  type UpdateView,
} from './engine.js';
import { chordMarks, glyph, shapeMarks } from './glyph.js';
import {
  buildKeyCodes,
  FINGER_NAMES,
  keyIndexFor,
  keyLegend,
  LEFT_THUMB_CHOICES,
  RIGHT_THUMB_CHOICES,
} from './keymap.js';
import { loadSettings, type Settings, saveSettings } from './settings.js';
import { countWords, formatSeconds, plural, wordsPerMinute } from './stats.js';

const PLANES: readonly {
  id: PlaneName;
  name: string;
  how: string;
  left: boolean;
  right: boolean;
}[] = [
  { id: 'core', name: 'Core', how: 'no thumbs', left: false, right: false },
  { id: 'extended', name: 'Extended', how: 'right thumb', left: false, right: true },
  { id: 'literal', name: 'Literal', how: 'left thumb', left: true, right: false },
  { id: 'user', name: 'Yours', how: 'both thumbs', left: true, right: true },
];

/** How many earlier exchanges the decoder sees as context. */
const CONTEXT_TURNS = 3;
/** How many rows the tape keeps before dropping the oldest. */
const TAPE_ROWS = 300;

type Child = Node | string;

function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  attributes: Record<string, string> = {},
  ...children: Child[]
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  for (const [name, value] of Object.entries(attributes)) {
    node.setAttribute(name, value);
  }
  node.append(...children);
  return node;
}

function need<T extends HTMLElement>(id: string): T {
  const node = document.getElementById(id);
  if (!node) {
    throw new Error(`The page is missing #${id}.`);
  }
  return node as T;
}

function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

/** A label for an action, wrapped so each kind can be styled. */
function actionLabel(entry: EntryView): HTMLElement {
  if (entry.kind === 'text') {
    return el('span', { class: 'label label-text' }, entry.label);
  }
  return el('span', { class: `label label-${entry.kind}` }, entry.label);
}

/** One-hand chord notation for a spelling shape, typed with the right hand. */
function rightHandChord(shape: string): string {
  return `-${[...'TIMRP'].filter((finger) => shape.includes(finger)).join('')}`;
}

export function start(engine: Engine): void {
  const layout = engine.layout();
  const families = new Map<string, FamilyView>(
    layout.families.map((family) => [`${family.plane}:${family.left}`, family]),
  );
  const storage = browserStorage();

  let settings: Settings = loadSettings(storage);
  let codes = buildKeyCodes(settings.leftThumb, settings.rightThumb);
  let legends: ReadonlyMap<string, string> | undefined;
  let update: UpdateView = engine.state();
  let browsePlane: PlaneName = 'core';
  let browseLeft: string | null = null;
  const latched = new Set<number>();
  const history: Turn[] = [];
  let strokeCount = 0;
  let firstStrokeAt: number | null = null;

  const bufferBox = need('buffer');
  const previewBox = need('preview');
  const latchBox = need('latch');
  const planesBox = need('planes');
  const familyBox = need('family-view');
  const alphabetBox = need('alphabet');
  const tapeList = need<HTMLOListElement>('tape');
  const tapeEmpty = need('tape-empty');
  const resultBox = need('result');
  const resultText = need('result-text');
  const keyButtons: HTMLButtonElement[] = [];
  const alphabetCells = new Map<string, HTMLElement>();

  need('layout-name').textContent = `${layout.name} ${layout.version}`;

  // --- Applying engine updates -------------------------------------------

  function apply(next: UpdateView): void {
    update = next;
    if (next.outcome) {
      record(next.outcome);
    }
    render();
  }

  function strike(chord: string): void {
    apply(engine.strike(chord, performance.now()));
  }

  function record(outcome: OutcomeView): void {
    strokeCount += 1;
    firstStrokeAt ??= outcome.started_ms;
    printStroke(outcome);
    if (outcome.effect.type === 'send') {
      const duration = outcome.ended_ms - firstStrokeAt;
      const strokes = strokeCount;
      strokeCount = 0;
      firstStrokeAt = null;
      void expand(outcome.effect.wire, strokes, duration);
    } else if (update.buffer.items.length === 0) {
      // An empty buffer means the next stroke starts a new utterance.
      strokeCount = 0;
      firstStrokeAt = null;
    }
  }

  // --- Buffer and result -------------------------------------------------

  function renderBuffer(): void {
    const { items, word_open: wordOpen } = update.buffer;
    bufferBox.replaceChildren();
    if (items.length === 0) {
      bufferBox.append(el('span', { class: 'buffer-empty' }, 'Nothing entered yet.'));
      return;
    }
    items.forEach((item, index) => {
      if (item.kind === 'concept') {
        bufferBox.append(el('span', { class: 'token' }, item.text), ' ');
        return;
      }
      const open = wordOpen && index === items.length - 1;
      const word = el('span', { class: 'word' }, `“${item.text}`);
      word.append(open ? el('span', { class: 'caret', 'aria-label': 'still spelling' }) : '”');
      bufferBox.append(word, ' ');
    });
  }

  function showResult(text: string): void {
    resultText.textContent = text;
    resultBox.hidden = false;
  }

  need('copy-result').addEventListener('click', (event) => {
    const button = event.currentTarget as HTMLButtonElement;
    void navigator.clipboard.writeText(resultText.textContent ?? '').then(
      () => {
        button.textContent = 'Copied';
      },
      () => {
        button.textContent = 'Could not copy';
      },
    );
    window.setTimeout(() => {
      button.textContent = 'Copy';
    }, 1600);
  });

  for (const [id, command] of [
    ['do-undo', 'undo'],
    ['do-space', 'space'],
    ['do-send', 'send'],
  ] as const) {
    const thumb = layout.thumbs.find((entry) => entry.label === command);
    const button = need<HTMLButtonElement>(id);
    if (thumb) {
      button.title = `Chord: ${thumb.chord}`;
      button.addEventListener('click', () => strike(thumb.chord));
    } else {
      button.disabled = true;
    }
  }

  // --- The ten keys ------------------------------------------------------

  function buildHands(): void {
    const left = need('hand-left');
    const right = need('hand-right');
    left.replaceChildren();
    right.replaceChildren();
    keyButtons.length = 0;
    codes.forEach((code, index) => {
      const legend = keyLegend(code, legends);
      const finger = FINGER_NAMES[index] ?? '';
      const fingerWord = finger.split(' ')[1] ?? '';
      const button = el(
        'button',
        {
          type: 'button',
          class: `key key-${fingerWord}`,
          'aria-label': `${finger}, the ${legend} key`,
          'aria-pressed': 'false',
        },
        el('span', { class: 'key-legend' }, legend),
        el('span', { class: 'key-finger' }, fingerWord),
      );
      button.addEventListener('click', () => latch(index));
      keyButtons.push(button);
      (index < 5 ? left : right).append(button);
    });
  }

  function renderHands(): void {
    keyButtons.forEach((button, index) => {
      const bit = 1 << index;
      const down = (update.held & bit) !== 0;
      button.classList.toggle('is-down', down);
      button.classList.toggle('is-pending', !down && (update.pending.bits & bit) !== 0);
      button.setAttribute('aria-pressed', String(down));
    });
    latchBox.hidden = latched.size === 0;
  }

  function renderPreview(): void {
    const { chord, preview } = update.pending;
    previewBox.replaceChildren();
    if (!chord) {
      previewBox.append(
        'Hold a left-hand shape to open a family, or press one hand alone to spell.',
      );
      return;
    }
    previewBox.append(el('span', { class: 'chord' }, chord), ' ');
    if (!preview) {
      previewBox.append('has no meaning in this layout.');
    } else if (preview.kind === 'concept') {
      previewBox.append(
        'releases as ',
        actionLabel(preview),
        preview.gloss ? `, meaning: ${preview.gloss}.` : '.',
      );
    } else if (preview.kind === 'text') {
      previewBox.append('types ', actionLabel(preview), '.');
    } else {
      previewBox.append('runs ', actionLabel(preview), '.');
    }
  }

  // Clicking keys latches them, for mice, touch screens and keyboards that
  // cannot register many keys at once.
  function latch(index: number): void {
    if (latched.has(index)) {
      return;
    }
    latched.add(index);
    apply(engine.press(index, performance.now()));
  }

  need('latch-release').addEventListener('click', () => {
    const now = performance.now();
    const keys = [...latched];
    latched.clear();
    for (const key of keys) {
      apply(engine.release(key, now));
    }
  });

  need('latch-cancel').addEventListener('click', () => {
    latched.clear();
    apply(engine.releaseAll());
  });

  // --- Families ----------------------------------------------------------

  function renderFamilies(): void {
    const pending = update.pending;
    const thumbsHeld = pending.left_thumb || pending.right_thumb;
    const plane = PLANES.find((p) => p.id === (thumbsHeld ? pending.family_plane : browsePlane));
    if (!plane) {
      return;
    }
    const heldLeft = pending.left;
    const left = heldLeft || browseLeft || '';

    planesBox.replaceChildren(
      ...PLANES.map((option) => {
        const button = el(
          'button',
          { type: 'button', 'aria-pressed': String(option.id === plane.id) },
          option.name,
          el('span', { class: 'plane-how' }, option.how),
        );
        button.addEventListener('click', () => {
          browsePlane = option.id;
          browseLeft = null;
          render();
        });
        return button;
      }),
    );

    familyBox.replaceChildren();
    const family = left ? families.get(`${plane.id}:${left}`) : undefined;
    if (family) {
      familyBox.append(familyGrid(family, plane, heldLeft ? pending.right : null, !heldLeft));
      return;
    }
    if (left) {
      familyBox.append(
        el(
          'p',
          { class: 'family-note' },
          `No family uses that left-hand shape in the ${plane.name} plane.`,
        ),
      );
    }
    familyBox.append(familyList(plane));
  }

  function familyList(plane: (typeof PLANES)[number]): HTMLElement {
    const inPlane = layout.families.filter((family) => family.plane === plane.id);
    if (inPlane.length === 0) {
      return el(
        'p',
        { class: 'family-note' },
        'Nothing here yet. This plane is kept free for your own vocabulary.',
      );
    }
    const list = el('ul', { class: 'family-list' });
    for (const family of inPlane) {
      const sample = family.members
        .slice(0, 5)
        .map((member) => member.label)
        .join('  ');
      const button = el(
        'button',
        { type: 'button' },
        glyph(
          shapeMarks(family.left + (plane.left ? 'T' : ''), 'left', true),
          `left hand ${family.left}`,
        ),
        el('span', { class: 'family-name' }, family.name),
        el('span', { class: 'family-sample' }, sample),
      );
      button.addEventListener('click', () => {
        browsePlane = plane.id;
        browseLeft = family.left;
        render();
      });
      list.append(el('li', {}, button));
    }
    return list;
  }

  function familyGrid(
    family: FamilyView,
    plane: (typeof PLANES)[number],
    heldRight: string | null,
    browsing: boolean,
  ): HTMLElement {
    const box = el('div', { class: 'family' });
    const heading = el(
      'div',
      { class: 'family-heading' },
      glyph(
        shapeMarks(family.left + (plane.left ? 'T' : ''), 'left', true),
        `left hand ${family.left}`,
      ),
      el('h3', {}, family.name),
    );
    if (browsing) {
      const back = el('button', { type: 'button', class: 'quiet' }, 'All families');
      back.addEventListener('click', () => {
        browseLeft = null;
        render();
      });
      heading.append(back);
    }
    box.append(heading);

    const grid = el('div', { class: 'member-grid' });
    for (const shape of layout.finger_shapes) {
      const member = family.members.find((candidate) => candidate.right === shape);
      const picture = glyph(
        shapeMarks(shape + (plane.right ? 'T' : ''), 'right', true),
        `right hand ${shape}`,
      );
      if (!member) {
        grid.append(
          el('div', { class: 'member is-free' }, picture, el('span', { class: 'label' }, 'free')),
        );
        continue;
      }
      const cell = el('button', { type: 'button', class: 'member' }, picture, actionLabel(member));
      if (member.gloss) {
        cell.title = member.gloss;
      }
      if (heldRight === shape) {
        cell.classList.add('is-current');
      }
      cell.addEventListener('click', () => strike(member.chord));
      grid.append(cell);
    }
    box.append(grid);
    return box;
  }

  // --- Alphabet ----------------------------------------------------------

  function buildAlphabet(): void {
    alphabetBox.replaceChildren();
    alphabetCells.clear();
    for (const spell of layout.spelling) {
      const cell = el(
        'button',
        { type: 'button', class: 'letter' },
        glyph(shapeMarks(spell.shape, 'right', true), `shape ${spell.shape}`),
        el('span', { class: 'letter-text' }, spell.text),
      );
      if (spell.shifted && spell.shifted.toLowerCase() !== spell.text) {
        cell.append(el('span', { class: 'letter-shifted' }, spell.shifted));
      }
      cell.addEventListener('click', () => strike(rightHandChord(spell.shape)));
      alphabetCells.set(spell.shape, cell);
      alphabetBox.append(cell);
    }
  }

  function renderAlphabet(): void {
    const pending = update.pending;
    let current: string | null = null;
    if (pending.left && !pending.right) {
      current = (pending.left_thumb ? 'T' : '') + pending.left;
    } else if (pending.right && !pending.left) {
      current = (pending.right_thumb ? 'T' : '') + pending.right;
    }
    for (const [shape, cell] of alphabetCells) {
      cell.classList.toggle('is-current', shape === current);
    }
  }

  // --- Tape --------------------------------------------------------------

  function printRow(row: HTMLLIElement): void {
    tapeEmpty.hidden = true;
    tapeList.append(row);
    while (tapeList.childElementCount > TAPE_ROWS) {
      tapeList.firstElementChild?.remove();
    }
    row.scrollIntoView({ block: 'nearest' });
  }

  function printStroke(outcome: OutcomeView): void {
    const row = el('li', { class: 'tape-stroke' }, glyph(chordMarks(outcome.bits), outcome.chord));
    const effect = outcome.effect;
    if (!outcome.action) {
      row.append(el('span', { class: 'label label-none' }, 'no meaning'));
    } else {
      row.append(actionLabel(outcome.action));
      if (effect.type === 'ignored') {
        row.append(el('span', { class: 'tape-aside' }, 'nothing to do'));
      } else if (effect.type === 'key') {
        row.append(el('span', { class: 'tape-aside' }, 'key'));
      }
    }
    printRow(row);
  }

  async function expand(wire: string, strokes: number, durationMs: number): Promise<void> {
    const text = el('p', { class: 'sent-text is-waiting' }, 'Asking the AI…');
    const stats = el(
      'p',
      { class: 'sent-stats' },
      `${plural(strokes, 'stroke')} in ${formatSeconds(durationMs)}.`,
    );
    printRow(el('li', { class: 'tape-sent' }, el('p', { class: 'sent-wire' }, wire), text, stats));

    try {
      // Built before the request so a quick second send cannot change it.
      const messages = engine.decodeMessages(history.slice(-CONTEXT_TURNS), settings.language);
      const reply = await decode(settings, messages);
      const sentence = cleanReply(reply.text);
      history.push({ utterance: wire, text: sentence });
      text.textContent = sentence;
      text.classList.remove('is-waiting');
      showResult(sentence);

      const words = countWords(sentence);
      const pace = wordsPerMinute(words, durationMs);
      stats.textContent =
        `${plural(strokes, 'stroke')} in ${formatSeconds(durationMs)} became ${plural(words, 'word')}` +
        (pace === null ? '.' : `, which is ${pace} words a minute.`) +
        ` The AI took ${formatSeconds(reply.ms)}.`;
    } catch (error) {
      text.classList.remove('is-waiting');
      text.classList.add('is-problem');
      if (error instanceof DecodeError && error.kind === 'no-key') {
        const provider = providerById(settings.providerId);
        text.textContent = `Add a free ${provider.name} key in Setup to turn this into a sentence.`;
      } else {
        text.textContent = error instanceof Error ? error.message : String(error);
      }
    }
    text.scrollIntoView({ block: 'nearest' });
  }

  // --- Setup dialog ------------------------------------------------------

  const dialog = need<HTMLDialogElement>('setup');
  const providerSelect = need<HTMLSelectElement>('set-provider');
  const keyInput = need<HTMLInputElement>('set-key');
  const keyLink = need<HTMLAnchorElement>('key-link');
  const baseUrlRow = need('base-url-row');
  const baseUrlInput = need<HTMLInputElement>('set-base-url');
  const modelInput = need<HTMLInputElement>('set-model');
  const modelList = need<HTMLDataListElement>('model-list');
  const languageInput = need<HTMLInputElement>('set-language');
  const leftThumbSelect = need<HTMLSelectElement>('set-left-thumb');
  const rightThumbSelect = need<HTMLSelectElement>('set-right-thumb');
  const setupStatus = need('setup-status');

  providerSelect.append(
    ...PROVIDERS.map((provider) => el('option', { value: provider.id }, provider.name)),
  );

  function fillThumbChoices(): void {
    for (const [select, choices] of [
      [leftThumbSelect, LEFT_THUMB_CHOICES],
      [rightThumbSelect, RIGHT_THUMB_CHOICES],
    ] as const) {
      select.replaceChildren(
        ...choices.map((code) => el('option', { value: code }, keyLegend(code, legends))),
      );
    }
  }

  function showProvider(): void {
    const provider = providerById(providerSelect.value);
    need('provider-note').textContent = provider.note;
    baseUrlRow.hidden = provider.id !== 'custom';
    modelInput.placeholder = provider.defaultModel || 'model name';
    keyLink.hidden = provider.keyUrl === null;
    if (provider.keyUrl) {
      keyLink.href = provider.keyUrl;
      keyLink.textContent = `Get a free ${provider.name} key`;
    }
  }

  function formSettings(): Settings {
    return {
      providerId: providerSelect.value,
      baseUrl: baseUrlInput.value.trim(),
      apiKey: keyInput.value.trim(),
      model: modelInput.value.trim(),
      language: languageInput.value.trim() || 'English',
      leftThumb: leftThumbSelect.value,
      rightThumb: rightThumbSelect.value,
    };
  }

  async function refreshModels(): Promise<void> {
    try {
      const models = await listModels(formSettings());
      modelList.replaceChildren(...models.map((model) => el('option', { value: model })));
    } catch {
      // The picker is a convenience. Typing a model name always works.
      modelList.replaceChildren();
    }
  }

  need('open-setup').addEventListener('click', () => {
    fillThumbChoices();
    providerSelect.value = settings.providerId;
    baseUrlInput.value = settings.baseUrl;
    keyInput.value = settings.apiKey;
    modelInput.value = settings.model;
    languageInput.value = settings.language;
    leftThumbSelect.value = settings.leftThumb;
    rightThumbSelect.value = settings.rightThumb;
    setupStatus.textContent = '';
    showProvider();
    dialog.showModal();
    void refreshModels();
  });

  providerSelect.addEventListener('change', () => {
    modelInput.value = '';
    showProvider();
    void refreshModels();
  });
  keyInput.addEventListener('change', () => void refreshModels());

  dialog.addEventListener('close', () => {
    if (dialog.returnValue !== 'save') {
      return;
    }
    settings = formSettings();
    const stored = saveSettings(storage, settings);
    codes = buildKeyCodes(settings.leftThumb, settings.rightThumb);
    buildHands();
    render();
    if (!stored) {
      previewBox.textContent =
        'This browser would not store your settings. They last until you close the page.';
    }
  });

  // --- Keyboard ----------------------------------------------------------

  function typingElsewhere(target: EventTarget | null): boolean {
    if (dialog.open) {
      return true;
    }
    return (
      target instanceof HTMLElement &&
      (target.isContentEditable || ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName))
    );
  }

  window.addEventListener('keydown', (event) => {
    if (event.ctrlKey || event.metaKey || event.altKey || typingElsewhere(event.target)) {
      return;
    }
    const key = keyIndexFor(event.code, codes);
    if (key === null) {
      return;
    }
    event.preventDefault();
    if (!event.repeat) {
      apply(engine.press(key, performance.now()));
    }
  });

  window.addEventListener('keyup', (event) => {
    const key = keyIndexFor(event.code, codes);
    if (key === null || latched.has(key)) {
      return;
    }
    if ((update.held & (1 << key)) === 0) {
      return;
    }
    event.preventDefault();
    apply(engine.release(key, performance.now()));
  });

  // Key releases never arrive after the page loses focus, so drop the stroke.
  window.addEventListener('blur', () => {
    latched.clear();
    apply(engine.releaseAll());
  });

  // --- First paint -------------------------------------------------------

  function render(): void {
    renderBuffer();
    renderHands();
    renderPreview();
    renderFamilies();
    renderAlphabet();
  }

  buildHands();
  buildAlphabet();
  render();

  // Show what is really printed on the user's keys, where the browser can tell.
  const keyboard = (
    navigator as Navigator & {
      keyboard?: { getLayoutMap(): Promise<ReadonlyMap<string, string>> };
    }
  ).keyboard;
  keyboard?.getLayoutMap().then(
    (map) => {
      legends = map;
      buildHands();
      render();
    },
    () => {
      // Not available everywhere. The US legends stay.
    },
  );
}
