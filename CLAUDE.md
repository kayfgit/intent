# Intent

A ten-key chord input system: chords encode intent and an AI expands it to
text. Target user: developers talking to AI coding assistants. Read
[docs/DESIGN.md](docs/DESIGN.md) before changing behaviour. It holds the
reasoning, the chord grammar and the roadmap.

This is a from-scratch successor to an older Python prototype. Do not port
code from it.

## Commands

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p intent-cli -- check layouts/intent-v0.toml --strict
```

All four must pass before a commit. CI runs the same on Linux and Windows.

## Structure

- `crates/intent-core`: the engine. Pure: no I/O, no clock, no `unsafe`, no
  platform code. Hosts pass in timestamps and act on returned `Effect`s. Keep
  it this way so it can compile to WebAssembly and to firmware.
- `crates/intent-cli`: the `intent` tool. All logic sits behind
  `intent_cli::run`, which takes its output streams as arguments so tests do
  not spawn processes.
- `layouts/intent-v0.toml`: the shipped layout.

## Rules that are easy to break

- **Layout files never contain chords.** Tokens are listed in priority order
  and the compiler assigns shapes. Do not add a way to hand-write chords.
- **`FINGER_SHAPE_ORDER` and `SPELL_SHAPE_ORDER` are frozen.** They are part
  of the layout format. Changing them moves every chord a user has learned.
  Changing `EffortModel` weights is fine as long as the tables' tests pass.
- **Every chord has one meaning.** There is one lookup table. Never
  special-case a chord in engine code ahead of the layout.
- **Undo must stay exact.** The buffer is derived from the stroke log. New
  buffer-changing behaviour goes in as a new `Op`, never as direct mutation.
- **Concept tokens are unique** across the whole layout and match
  `[A-Z][A-Z0-9_]*`. One token covers noun and verb uses (`TEST`), and the
  decoder picks the part of speech.
- **No speed claims without measurements.** Docs state what was measured, or
  say it is unknown.

## Conventions

- Test names describe behaviour in plain words.
- Error messages say what is wrong and what was expected, and layout errors
  name the family and member.
- Changing the v0 layout's counts means updating the guard tests in
  `crates/intent-core/tests/v0_layout.rs` and the CLI tests.
- Update `CHANGELOG.md` with every user-visible change.
