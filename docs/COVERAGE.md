# How much everyday language intent-v0 covers

Measured on 2026-11-17 with `intent coverage` and `intent encode`. Every
number here comes from a run described below. The "if the free chords were
filled" table is the only estimate, and it is marked as one.

## Short answer

| | v0 0.1.0 | v0 0.2.0 (50 everyday words added) |
|---|---|---|
| Subtitle words needing no spelling | 45.0% | **66.0%** |
| Conversation words needing no spelling | 47.4% | **67.8%** |
| Conversation lines needing no spelling at all | 2.3% | **8.1%** |
| Strokes per word in conversations | 3.21 | **2.57** |
| Round trip: meaning kept / mostly kept / lost (40 lines) | 26 / 5 / 9 | **31 / 5 / 4** |
| Round trip on 30 fresh lines | | **25 / 2 / 3** |

- **Two thirds of everyday words now need no spelling.** Adding the 50 most
  often spelled words (*that, of, have, do, me, your…*) moved coverage by
  about 20 points, as predicted.
- **Whole sentences still mostly need some spelling**: only 8% avoid it. The
  long tail of nouns and names (*beer, hospital, Susan*) stays spelled.
- **The decoder keeps the meaning of about 4 in 5 lines.** Most of what it
  still gets wrong is **tense**: "It was hard" is entered as `IT "hard"` and
  comes back as "It is hard". The layout has no way to say "past" yet.

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
  or as a form of it ("fixed" → `FIX`, "went" → `GO`, "thank" → `THANKS`).
  Contractions are expanded first ("don't" → do + not). Pronoun case ("me"
  for `I`) and modals ("would" for `WILL`) do not count: the first round
  trip showed the decoder cannot restore them.
- **grammar**: an article or a form of "be". The decoder is expected to add
  these, so they cost no stroke.
- **spelled**: everything else.

The matching is simple suffix rules plus a small table of irregular verbs. It
errs both ways: it counts "fixed" as `FIX` although the past tense is lost
(see the round trip), and it counts as spelled a word the decoder could
have paraphrased ("wanna" for `WANT`). Clause-sized concepts such as
`WITH_TESTS` are not counted: they stand for a phrase, not a word.

## Coverage

| | Subtitles 0.1.0 | Subtitles 0.2.0 | DailyDialog 0.1.0 | DailyDialog 0.2.0 |
|---|---|---|---|---|
| Concept | 33.0% | 54.0% | 35.6% | 56.0% |
| Grammar | 12.0% | 12.0% | 11.8% | 11.8% |
| Spelled | 55.0% | 34.0% | 52.6% | 32.2% |
| **No spelling needed** | **45.0%** | **66.0%** | **47.4%** | **67.8%** |
| Lines with no spelling at all | | | 2.3% | 8.1% |
| Strokes per word | | | 3.21 | 2.57 |
| Keystrokes per word on a keyboard | | | 4.88 | 4.88 |

Strokes per word counts one stroke per concept, one per spelled letter, a
space stroke between spelled words and one send per line. Keystrokes count
letters plus one space. A chord takes longer to press than a key, so fewer
strokes does not mean faster; that needs timed sessions.

### What 0.2.0 added

Four families in the extended plane (right thumb held), on the easiest
left-hand shapes that were free, each ordered by frequency:

| Family | Left hand | Members |
|---|---|---|
| Little words | P | that, of, on, there, just, at, here, about, up, out, as, too, much, by, back |
| Everyday verbs | IM | do, have, would, get, go, see, come, take, could, look, tell |
| Pronouns | MR | me, your, my, he, she, him, her, our, them, who, his, us |
| Everyday words | IR | well, good, right, one, time, really, very, some, sorry, any, people, two |

These were the 50 most often spelled words across both corpora, leaving out
interjections (*oh, yeah, hey*), forms of words that already had a token
(*did, going, got*) and forms of address (*sir, mr*). The existing families
were not moved, so these frequent words sit on thumb chords while some rarer
developer words keep thumb-free chords. Re-ranking every family by measured
frequency is a separate change.

### What is still missing

The most frequent spelled words in DailyDialog with 0.2.0: *oh, great,
help, day, work, yeah, may, way, sir, many, nice, company, lot, something,
anything*, each 0.1% to 0.3% of words. The curve is now flat: no single word
is worth much.

### If the free chords were filled

An estimate, not a measurement: the share needing no spelling if the N most
frequent spelled words each got a chord. It ignores that a word may not suit
a chord and that several forms of one word would share a token.

| Added words | Subtitles | DailyDialog |
|---|---|---|
| 0 (0.2.0 today) | 66.0% | 67.8% |
| 50 | 71.3% | 72.6% |
| 100 | 74.0% | 75.4% |
| 200 | 77.3% | 78.9% |
| 400 | 81.1% | 83.3% |
| 597 (every free chord) | 83.3% | 86.0% |

For 0.1.0 the same table predicted 64.8% and 66.6% after 50 words; the
measured result was slightly higher because forms such as *did* and *going*
are covered by the new base words.

## Round trip through the decoder

Coverage says which words have a chord. It does not say whether the decoder
turns the result back into the sentence the user meant. Random DailyDialog
lines of 4 to 14 words were encoded with `intent encode` (concept tokens,
spelled words, grammar words dropped), decoded by `openai/gpt-oss-120b` on
Groq with the demo's prompt, and compared with the original by the same model
acting as a judge.

| Run | Layout and prompt | Same meaning | Detail lost | Meaning lost |
|---|---|---|---|---|
| 1 (40 lines) | 0.1.0, first prompt | 26 | 5 | 9 |
| 2 (same 40 lines) | 0.2.0, fixed prompt | 31 | 5 | 4 |
| 3 (30 new lines) | 0.2.0, fixed prompt | 25 | 2 | 3 |

Decoding took about 0.65 s at the median in every run.

Run 3 matters because the prompt fixes were designed from the failures in
run 1. On lines the fixes never saw, the result held at about the same
level, so the fixes were not just tuned to the test lines.

The judge is the same model as the decoder and the samples are small: treat
the splits as rough.

### What run 1 found, and what changed

| Cause in run 1 | Lines | Fix |
|---|---|---|
| "would" encoded as `WILL` | 3 | Matcher no longer maps modals; 0.2.0 has `WOULD` and `COULD` |
| Pronoun case lost ("my number" → "a number") | 4 | Matcher no longer maps case; 0.2.0 has `ME`, `MY`, `HIM`… |
| A dropped "is" or "am" not restored | 4 | Prompt now says to restore articles and "be", with an example |
| A question read as a statement | 1 | Prompt now says when to write a question |
| "news" read as the plural of `NEW` | 1 | Matcher exception |
| Literal words printed with their quote marks | 3 | Prompt now forbids quote marks (2 of 70 later replies still had them) |

### What runs 2 and 3 still get wrong

Counting every line in runs 2 and 3 that was not judged the same (14 lines):

| Cause | Lines | Example |
|---|---|---|
| Tense lost | 8 | "What did you do during the interview?" → `WHAT DO YOU DO…` → "What do you do during an interview?" |
| "a" came back as "the" or nothing | 3 | "I'd love a gin and tonic." → "I would love gin and tonic." |
| A question lost its question form | 1 | "Of course! Are you?" → "Of course, you." |
| An initial read as a contraction ("D.H." → "d" → `WOULD`) | 1 | Matcher bug in the test, not the layout |
| A line that was garbled to begin with | 1 | "Sure thing, follow me. This here is the." |

Tense is now the main problem, and it comes from the design rather than the
decoder: past-tense words ("did", "was", "fixed") are entered as their base
concept, so the past is never typed. A user cannot say it with v0 today.
Steno solves this with suffix strokes for -ed, -ing and -s. The equivalent
here would be a few modifier concepts such as `PAST` and `ONGOING`, or a
tense shape on the thumbs.

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
spaced punctuation joined back ("it ' s" → "it's"). The round-trip script is
not in the repository yet.
