# How much everyday language intent-v0 covers

Measured on 2026-11-17 with `intent coverage` and `intent encode`. Every
number here comes from a run described below. None are estimates, except the
"if the free chords were filled" column, which is marked as a ceiling.

## Short answer

- **A bit under half of the words in everyday speech need no spelling** with
  intent-v0: 45% of subtitle words and 47% of words in everyday
  conversations. A third are concepts; the rest of that half are articles and
  forms of "be" that the decoder is meant to add. Everything else is spelled
  letter by letter.
- **Almost no whole sentence avoids spelling**: 2.3% of conversation lines
  (180 of 7,740).
- **The gap is mostly small, common words**, not rare ones. v0 was built for
  talking to coding assistants and lacks *have, that, do, of, your, me, my,
  on, would, at, there, about*. Fifty more words would lift coverage to about
  two thirds.
- **When 40 random lines went through the real decoder, the meaning survived
  in 26, mostly survived in 5 and was lost in 9.** Every failure had a
  fixable cause, listed below.

## What was measured

### Corpora

| Corpus | What it is | Size |
|---|---|---|
| Subtitles | The 50,000 most frequent words in English film and TV subtitles ([OpenSubtitles 2018 via FrequencyWords](https://github.com/hermitdave/FrequencyWords)), weighted by how often each occurs | 727 million word occurrences |
| DailyDialog | Everyday written conversations ([DailyDialog](https://aclanthology.org/I17-1099/), test split) | 7,740 lines, 91,927 words |

Neither corpus is in the repository (their licences are share-alike and
non-commercial). Both are public downloads.

Neither is developer speech. A matching corpus of prompts written to coding
assistants is still needed to measure what v0 was designed for.

### How a word is classified

Each word is one of:

- **concept**: a concept token covers it, either directly ("fix" → `FIX`)
  or as a form the decoder can produce ("fixed" → `FIX`, "went" → `GO`,
  "thank" → `THANKS`). Contractions are expanded first ("don't" → do + not).
  Pronoun case ("me" for `I`) and modals ("would" for `WILL`) do not count:
  the round trip showed the decoder cannot restore them.
- **grammar**: an article or a form of "be". The decoder adds these from
  grammar, so they cost no stroke.
- **spelled**: everything else.

The matching is simple suffix rules plus small tables of irregular verbs and
pronouns. It can accept a form the decoder would get wrong, and it counts as
spelled a word the decoder could have paraphrased ("wanna" for `WANT`), so it
errs both ways. Clause-sized concepts such as `WITH_TESTS` are not counted:
they stand for a phrase, not a word.

## Results

| | Subtitles | DailyDialog |
|---|---|---|
| Concept | 33.0% | 35.6% |
| Grammar | 12.0% | 11.8% |
| Spelled | 55.0% | 52.6% |
| **No spelling needed** | **45.0%** | **47.4%** |
| Lines with no spelling at all | | 2.3% |
| Strokes per word | | 3.21 |
| Keystrokes per word on a keyboard | | 4.88 |

Strokes per word counts one stroke per concept, one per spelled letter, a
space stroke between spelled words and one send per line. Keystrokes count
letters plus one space. A chord takes longer to press than a key, so fewer
strokes does not mean faster; that needs timed sessions.

### What is missing

The most frequent spelled words in DailyDialog, with their share of all
words:

| Word | Share | Word | Share | Word | Share |
|---|---|---|---|---|---|
| have | 1.4% | my | 0.6% | good | 0.4% |
| that | 1.4% | on | 0.6% | he | 0.4% |
| do | 1.3% | would | 0.6% | just | 0.4% |
| of | 1.2% | at | 0.5% | go | 0.4% |
| your | 0.7% | there | 0.4% | here | 0.3% |
| me | 0.7% | about | 0.4% | get | 0.3% |
| | | well | 0.4% | one | 0.3% |

Most are function words (prepositions, pronouns, auxiliaries) and the most
common verbs and adjectives. They are cheap to add: v0 uses 376 of 1,023
chords, leaving 647 free (75 in the core plane, 180 extended, 167 literal,
225 user).

### If the free chords were filled

A ceiling, not a measurement: the share that would need no spelling if the
N most frequent spelled words each got a chord. It ignores that a word may
not suit a chord and that several forms of one word would share a token.

| Added words | Subtitles | DailyDialog |
|---|---|---|
| 0 (v0 today) | 45.0% | 47.4% |
| 50 | 64.8% | 66.6% |
| 100 | 70.0% | 71.4% |
| 200 | 74.9% | 76.4% |
| 400 | 79.8% | 81.8% |
| 647 (every free chord) | 82.9% | 85.6% |

Gains flatten fast: the first 50 words add about 20 points, the next 550
add about 16. The long tail (names, places, rare words) stays
spelled whatever the layout does, which is why spelling speed matters.

## Round trip through the decoder

Coverage says which words have a chord. It does not say whether the decoder
turns the result back into the sentence the user meant. To check, 40 random
DailyDialog lines of 4 to 14 words were encoded with `intent encode` (concept
tokens, spelled words, grammar words dropped), decoded by
`openai/gpt-oss-120b` on Groq with the demo's prompt, and then compared with
the original by the same model acting as a judge.

| Verdict | Lines |
|---|---|
| Same meaning | 26 |
| Main point kept, a detail lost | 5 |
| Meaning lost | 9 |

Decoding took 0.66 s at the median and 1.03 s at worst.

This run used an earlier version of the matching that still counted pronoun
case and modals as concepts, so it encoded "I'd" as `I WILL` and "me" as `I`.
That earlier version is why the coverage numbers above are lower than a first
draft: the stricter rules came from this test.

### Why meaning was lost

Lines counts every line that showed the problem, whatever its verdict.

| Cause | Lines | Example |
|---|---|---|
| "would" encoded as `WILL` | 3 | "I'd love a gin and tonic." → "I will love gin and tonic." |
| Pronoun case lost | 4 | "Do you have my number?" → "Do you have a number?" |
| A dropped "is" or "am" not restored | 4 | "I am impressed." → "I impressed." |
| A question read as a statement | 1 | "When are you getting married?" → "When you are getting married." |
| A suffix rule misread a word | 1 | "great news" → `NEW` → "great new" |
| Literal words copied with their quote marks or capitals | 3 | → "I "do" not "understand" why…" |

Some lines had more than one cause. The first, second and fifth are fixed
in the matching now. The rest need decoder prompt changes: restore "be" and
articles even next to literal words, never print quote marks, and treat an
input that starts with a question word as a question. Decoder behaviour was
not tuned for this report, so these numbers are a baseline.

The judge is the same model as the decoder and 40 lines is a small sample:
treat the split as rough. A repeat run after the fixes, with more lines, is
the next step.

## Reproduce

```bash
cargo run -p intent-cli -- coverage layouts/intent-v0.toml en_50k.txt --frequency
```

```bash
cargo run -p intent-cli -- coverage layouts/intent-v0.toml dailydialog_test.txt
```

```bash
cargo run -p intent-cli -- encode layouts/intent-v0.toml sample.txt --messages
```

DailyDialog was flattened to one line per utterance, with the dataset's
spaced punctuation joined back ("it ' s" → "it's").
