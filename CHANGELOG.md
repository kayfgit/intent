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
