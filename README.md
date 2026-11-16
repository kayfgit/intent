# Intent

A ten-key chord input system. You press a few keys together to say what you
mean, and an AI turns it into finished text. Built first for developers
talking to AI coding assistants.

```
strokes:  FIX  BUG  IN  l o g i n  WITH_TESTS  <send>
sent:     FIX BUG IN "login" WITH_TESTS
result:   Fix the bug in login and add tests that cover it.
```

## Status

Early. Milestone 1 of 7 is done: the core engine, the layout format, the v0
layout and the command-line tools. There is **no AI expansion and no keyboard
capture yet**, so you cannot type with it today. See the
[roadmap](docs/DESIGN.md#roadmap).

## How it works

Ten keys, one per finger. The shape of a chord decides its job:

- **One hand spells.** Thirty shapes, the same on both hands, cover the
  alphabet. Add the opposite thumb for a capital.
- **Two hands mean something.** The left hand picks a family (Actions,
  Things, Logic…) and the right hand picks a member (`MAKE`, `FIX`…).
- **Thumbs change the plane.** Right thumb for more concepts, left thumb for
  digits, symbols and arrow keys. Thumbs alone are undo, space and send.

Chords are written left hand, hyphen, right hand, using `P R M I T` for pinky,
ring, middle, index and thumb. `I-M` is left index plus right middle.

The full reasoning is in [docs/DESIGN.md](docs/DESIGN.md).

## Try the tools

Requires [Rust](https://rustup.rs) 1.88 or newer.

Validate the layout:

```bash
cargo run -p intent-cli -- check layouts/intent-v0.toml
```

See how the chord space is used:

```bash
cargo run -p intent-cli -- stats layouts/intent-v0.toml
```

See the strokes for a phrase:

```bash
cargo run -p intent-cli -- how layouts/intent-v0.toml FIX BUG IN login WITH_TESTS "<send>"
```

Apply chords and see the buffer:

```bash
cargo run -p intent-cli -- sim layouts/intent-v0.toml I-M M-I -T
```

## Editing the layout

[layouts/intent-v0.toml](layouts/intent-v0.toml) is plain TOML. You never
write chords in it. You list tokens in priority order and the easiest shapes
go to whatever comes first. Run `check` after any change: it reports every
problem in one pass.

## Development

```bash
cargo fmt --all
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

```bash
cargo test --workspace
```

CI runs all three plus a strict layout check on Linux and Windows.

## Project layout

```
crates/intent-core/   pure engine: keys, chords, planes, effort, strokes, layout, engine
crates/intent-cli/    the `intent` command-line tool
layouts/              layout files
docs/DESIGN.md        design, research notes, roadmap
```

## License

MIT
