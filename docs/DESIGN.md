# Intent: design

Intent is a ten-key chord input system. You press a few keys together to say
what you mean, and an AI turns that into finished text. It is built first for
developers talking to AI coding assistants.

This document explains why it is designed the way it is, what exists today,
and what comes next.

## The idea

Typing spells out every character of a sentence. Most of those characters
carry no new information: the reader, or a language model, could have guessed
them. Intent lets you enter only the part that could not be guessed.

```
strokes:  FIX  BUG  IN  l o g i n  WITH_TESTS  <send>
sent:     FIX BUG IN "login" WITH_TESTS
result:   Fix the bug in login and add tests that cover it.
```

Ten strokes replace about fifty keystrokes. The goal is not to move fingers
faster. It is to need fewer movements per idea.

## What the first prototype taught

Intent restarts an earlier Python prototype, the Universal Chord Keyboard. The
prototype proved the idea was usable and exposed where it broke.

| Prototype | What went wrong | Intent |
|---|---|---|
| About 450 words, each on a hand-written chord | 5 common words were unreachable because control chords shadowed them, and 24 words appeared twice | Layout files never contain chords. Tokens are listed by priority and the compiler assigns shapes, so collisions and shadowing cannot be written |
| Words outside the list needed a switch to QWERTY | No path to a ten-key wearable | Spelling lives on the same ten keys, as one-hand chords |
| One token per English word | Little real compression | Clause-sized concepts (`WITH_TESTS`, `PLAN_FIRST`) with a gloss for the AI |
| Controls checked before vocabulary in code | Order-dependent bugs | One table. Every chord has at most one meaning |
| Web demo and desktop app had different layouts | Drift | One layout file is the single source for every host |
| Never measured | Nobody knew if it was faster than typing | Stroke timing is built into the engine, and a trainer with real numbers is a milestone, not an afterthought |
| No tests | Regressions | The engine is pure and covered by unit and property tests |

Kept from the prototype: ten keys with one per finger, the left hand choosing
a category and the right hand choosing within it, lossy input with an AI
finishing the job, and the focus on developers.

## What the research says

A short reading of the text-entry literature shaped four decisions.

**The limit is the person, not the device.** Models of expert typing put
keyboard-style entry at roughly 58 to 67 words per minute before composing the
sentence, rather than typing it, becomes the bottleneck
([Soukoreff and MacKenzie](https://www.yorku.ca/mack/bit95.html),
[Kristensson and Vertanen 2014](https://www.keithv.com/pub/inviscid/KristenssonVertanenMobileHCI2014.pdf)).
Going beyond that means entering less, not pressing faster.

**Speed and ease of learning pull against each other.** Chord systems that are
quick to learn settle around 20 to 50 words per minute
([Twiddler study](https://sites.cc.gatech.edu/home/thad/p/030_10_MTE/twiddler-iswc.pdf)).
Systems above 200, such as stenotype, cost years of memorising. Intent tries
to ease the trade with structure you can reason about, hints on screen while
you hold keys, and an AI that tolerates rough input.

**Layouts can be computed.** KALQ searched millions of assignments against a
model of thumb movement
([Oulasvirta et al. 2013](https://keithv.com/pub/twothumb/OulasvirtaEtAlCHI2013.pdf)),
and the same approach helped design the French national keyboard standard
([Feit et al. 2021](https://cacmb4.acm.org/magazines/2021/2/250082-azerty-amlior)).
Intent's layout is data so that it can be re-fitted the same way.

**A language model can absorb imprecision.** Dasher
([Ward and MacKay](https://arxiv.org/pdf/cs/0204030)) and swipe keyboards
([Zhai and Kristensson](https://cacmb4.acm.org/magazines/2012/9/154575-the-word-gesture-keyboard))
both let the user be vague and let a model resolve it. Modern language models
make that far stronger than it was when those systems were built.

## Principles

1. **Enter intent, not characters.** One stroke should carry a word or a whole
   clause.
2. **Nothing is out of reach.** Any word can be spelled on the same ten keys.
3. **Structure beats memorising.** A chord's job follows from its shape, and
   the screen shows what the held keys can become.
4. **Every stroke can be taken back.** Undo is exact, always.
5. **The layout is data.** It is validated by tools and meant to be re-fitted
   from real usage.
6. **Measure, then believe.** Speed claims come from recorded sessions only.

## The chord grammar

### Keys and notation

There are ten keys, one per finger, named by finger so the layout does not
depend on any keyboard: `P` pinky, `R` ring, `M` middle, `I` index, `T` thumb.

A chord is written as the left hand in board order, a hyphen, then the right
hand: `PRMIT-TIMRP`. So `I-I` is both index fingers and `-T` is the right
thumb alone.

### Planes

The shape of a chord decides its job. Every one of the 1023 possible chords
falls into exactly one plane, and code checks this.

| Plane | Shape | Job | Chords |
|---|---|---|---|
| Thumbs | thumbs only | undo, space, send | 3 |
| Spell | fingers of one hand, optionally that hand's thumb | letters | 30 per hand |
| Shift | a spelling chord plus the opposite thumb | capitals | 30 per hand |
| Core | fingers of both hands | everyday concepts | 225 |
| Extended | both hands plus right thumb | more concepts | 225 |
| Literal | both hands plus left thumb | digits, symbols, keys | 225 |
| User | both hands plus both thumbs | your own vocabulary | 225 |

The rule of thumb: **one hand spells, two hands mean something, thumbs change
the plane.**

### Families and members

In the two-hand planes the left hand picks a family and the right hand picks a
member. Hold left index and you are in Actions: add right index for `MAKE`,
right middle for `FIX`, and so on. Each family holds up to fifteen members.

### How slots are assigned

A layout file lists members in priority order. The format hands out right-hand
shapes from easiest to hardest:

```
I  M  R  P  IM  MR  IR  RP  MP  IMR  IP  MRP  IMP  IRP  IMRP
```

That order comes from a small effort model: each finger has a cost, each extra
key costs a little, and a lifted finger between two pressed ones costs more.
The weights are a starting guess. The order itself is frozen as part of the
layout format, because changing it would move every chord under the user's
fingers.

### Spelling

Both hands carry the same thirty-symbol map, so there are thirty shapes to
learn and either hand can type any letter. Alternating hands keeps spelling
flowing. The four single-finger chords are the four most common letters.

Spelling is the fallback, not the main road. It only has to be good enough for
names, identifiers and rare words. Spelled words are passed to the AI as
literal text and copied exactly, so an identifier is never "corrected".
Letting the AI expand shortened spellings is an open question.

## Architecture

```
key events ─► StrokeDetector ─► Engine ─► Effect ─► host
                                  ▲          │
                               Layout        └─► Send(utterance) ─► decoder ─► text
```

- **`intent-core`** is pure: no I/O, no clock, no unsafe code. It holds keys,
  chords, planes, the effort model, the stroke detector, the layout compiler
  and the engine. Because it is pure it can later compile to WebAssembly for a
  web trainer or run on a wearable's microcontroller.
- **`intent-cli`** is the tool for working on layouts: `check`, `stats`, `sim`
  and `how`.
- **`intent-wasm`** exposes the engine to web pages as WebAssembly.
- **Hosts** capture real keys, show hints, call the decoder and deliver the
  result. The web demo in `web/` is the first one. A desktop host that types
  into other apps is not built yet.

The engine keeps a log of strokes and derives the buffer from it. That is why
undo is exact: it removes the last log entry and nothing else.

The utterance has a plain wire format. Concepts are bare uppercase tokens and
spelled words are quoted, so they can never be confused:
`RENAME "foo" TO "bar"`.

## The decoder

The decoder is any chat model that turns an utterance into a sentence. The
core builds the messages (`intent_core::prompt`): short instructions, four
worked examples, a few earlier turns for context, then the utterance with the
meaning of any clause-sized concepts it uses. The host sends them. The fixed
part of the prompt is kept near 400 tokens because free tiers count tokens.

### Choosing a free provider

Researched in November 2026. An input method needs a reply in well under a
second, limits that survive a practice session, and an API a web page can
call directly.

| Provider | Free tier | Catch |
|---|---|---|
| **Groq** (default) | No card. About 30 requests a minute and 1,000 a day per model, with daily token caps | The daily token cap allows a few hundred decodes a day |
| Google Gemini | Flash and Flash-Lite models are free | Google may use free-tier content to improve its products, and no longer publishes the free limits |
| Cerebras | Sources disagree on whether a lasting free tier still exists | Reported 5 requests a minute on the trial, too few for typing |
| OpenRouter | Free model variants | 50 requests a day unless you buy credits |
| Local (Ollama and similar) | Free and private | Needs a capable machine and setup |

Groq is the default because its limits are published and stable, it needs no
card, it is fast (a decode with `openai/gpt-oss-120b`, the default model,
took 0.6 to 0.8 s when checked), and its data page says
inference data is not retained by default, apart from temporary logs for
abuse monitoring. The four hosted providers answer browser requests: their
APIs returned permissive CORS headers when checked.

Groq removed `llama-3.3-70b-versatile`, the first default, in November 2026.
gpt-oss is a reasoning model, so the demo asks it for low reasoning effort,
which cut one decode from 128 to 51 output tokens without changing the
answer.

Every provider above speaks the same OpenAI-style chat format, so the demo
has one small client and the provider is a setting. Limits change often;
the numbers here will go stale.

Sources: [Groq models](https://console.groq.com/docs/models),
[Groq rate limits](https://console.groq.com/docs/rate-limits),
[Groq data handling](https://console.groq.com/docs/your-data),
[Gemini pricing](https://aistudio.google.com/docs/pricing),
[OpenRouter's comparison of free tiers](https://openrouter.ai/blog/tutorials/free-llm-apis-compared/),
[Cerebras rate limits](https://inference-docs.cerebras.ai/support/rate-limits).

### What is not known yet

The prompt has never been scored against real models. There is no test set
of utterances with expected sentences, so nobody knows how often the decoder
gets it right, or how the choice of model changes that. Building that test
set is the next step of this milestone.

## Roadmap

| Milestone | What it delivers | Status |
|---|---|---|
| 1. Foundation | Core engine, layout format and compiler, v0 layout, CLI, CI | Done |
| 2. Decoder | Turn an utterance into text with a language model. Context from earlier turns, several candidates, one-stroke correction, a latency budget, an offline test set to score layouts and prompts | Started: prompt builder and provider client work. No test set, candidates or correction yet |
| 3. Windows host | Global key capture, live hint overlay, typing into any app | Planned |
| 4. Trainer | Drills with spaced repetition. Records words per minute, strokes per word, correction rate and learning curves | Planned |
| 5. Fitted layout | Re-fit the effort weights and the token order from recorded sessions and a corpus of real prompts | Planned |
| 6. Web demo | The same engine in the browser through WebAssembly | First version done: live hints, families, alphabet, tape, AI expansion |
| 7. Hardware | A ten-key wearable or split board speaking the same key events | Planned |

## Open questions

These are known unknowns. Each needs an experiment, not an opinion.

- **Rolled strokes.** The detector waits for all keys to lift. Fast
  alternating one-hand letters could overlap and merge into a two-hand chord.
  Steno has the same rule and experts cope, but a timing-aware detector may be
  kinder to beginners.
- **Syllable spelling.** One letter per stroke is slow. A plane where the left
  hand gives a consonant and the right a vowel would double that. It is
  deferred until measurements show spelling is the bottleneck.
- **Is the effort model right?** The weights are educated guesses. The trainer
  will record time and errors per chord so they can be fitted.
- **How lossy is too lossy?** Fewer strokes means more guessing by the AI and
  more corrections. The decoder milestone needs a test set to find the
  balance.
- **How much does v0 cover?** A bit under half of everyday spoken words need no
  spelling; see [COVERAGE.md](COVERAGE.md). Developer prompts, the target,
  are not measured yet.
- **Which letters first?** v0 blends English and Portuguese letter frequency.
  A corpus of real prompts should decide.
- **Does it beat typing?** Unknown until milestone 4. The honest baseline is
  the user's own QWERTY speed on the same prompts, counted in finished words
  per minute including corrections.
