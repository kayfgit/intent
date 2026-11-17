# Changelog

All notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `intent-core`, a pure engine with no I/O:
  - Ten finger-named keys and steno-style chord notation (`PRMIT-TIMRP`).
  - A plane grammar that gives every one of the 1023 chords exactly one job.
  - An effort model that ranks hand shapes from easiest to hardest.
  - A stroke detector that turns timestamped key events into chords.
  - A layout format in TOML where tokens are listed by priority and chords
    are assigned automatically, with a compiler that reports every problem
    in one pass.
  - An engine that builds an utterance from strokes, with exact undo.
- `intent-v0`, a seed layout: 195 concepts in 13 families, a mirrored
  one-hand alphabet with capitals, digits, symbols and navigation keys.
- `intent` command-line tool with `check`, `stats`, `sim` and `how`.
- Design document covering the research background, chord grammar,
  architecture and roadmap.
- CI on Linux and Windows: formatting, lints, tests and a strict layout check.
- A web demo (`web/`): press chords on the home row, see what each chord
  will do before releasing it, browse families and the alphabet, and read a
  tape of every stroke. Works with the mouse too.
- AI expansion in the demo through any OpenAI-compatible provider, with the
  user's own key kept in the browser. Groq is the default; Gemini and custom
  or local servers are built in.
- `intent-core`: a `Session` that joins the stroke detector and the engine,
  and a `prompt` module that builds the messages a decoder model receives.
- `intent-wasm`: a WebAssembly bridge exposing the engine to web pages.
- `intent coverage`: measures how much of a corpus a layout covers with
  concepts, and lists the most frequent words that still need spelling.
- `intent encode`: shows what a user would enter for each sentence, with
  the decoder messages as JSON for round-trip tests.
- `docs/COVERAGE.md`: intent-v0 measured on everyday English.

### Changed

- intent-v0 0.2.0 adds the 50 words most often spelled in everyday English
  (that, of, have, do, me, your…) as four extended-plane families. Words
  needing no spelling rose from 47% to 68% in everyday conversations.
- The decoder prompt now says to restore dropped articles and forms of
  "be", never to print quote marks, and when to write a question.

- The demo's default model is now `openai/gpt-oss-120b`, asked for low
  reasoning effort. Groq removed `llama-3.3-70b-versatile`, so the old
  default failed with "model does not exist".

- The Rust toolchain is pinned in `rust-toolchain.toml`, so local runs and
  CI use the same compiler and lints.
