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

Early. The core engine, the layout format, the v0 layout and the
command-line tools are done, and there is a **web demo you can try**. The
demo works inside its own page only: it cannot type into other apps yet. See
the [roadmap](docs/DESIGN.md#roadmap).

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

## Try the demo

Requires [Rust](https://rustup.rs), [wasm-pack](https://rustwasm.github.io/wasm-pack/),
Node 22 and [pnpm](https://pnpm.io).

```bash
pnpm --dir web install
```

```bash
pnpm --dir web dev
```

Then open <http://localhost:5173>.

- Fingers rest on `A S D F` and `J K L` plus the key after it. Thumbs use `C`
  and `M` (changeable in Setup).
- Hold keys and the page shows what the chord will do before you release it.
  The tape on the side prints every chord you press.
- Send with both thumbs. Without an API key you see the compressed form only.
  Add a free [Groq](https://console.groq.com/keys) key in Setup to get full
  sentences. The key stays in your browser and goes only to the provider.
- If your keyboard drops keys in big chords, click the on-screen keys and
  press Release instead.

## Try the tools

Requires [Rust](https://rustup.rs). The exact version is pinned in
`rust-toolchain.toml` and installs itself on first use.

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

Measure how much of a text file the layout covers (one sentence per line):

```bash
cargo run -p intent-cli -- coverage layouts/intent-v0.toml corpus.txt
```

Results for everyday English are in [docs/COVERAGE.md](docs/COVERAGE.md).

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

For the web demo, from the `web` folder: `pnpm lint`, `pnpm typecheck`,
`pnpm test` and `pnpm build`.

CI runs all of it: the Rust checks plus a strict layout check on Linux and
Windows, and the web checks on Linux.

## Project layout

```
crates/intent-core/   pure engine: keys, chords, planes, effort, strokes, layout, engine
crates/intent-cli/    the `intent` command-line tool
crates/intent-wasm/   WebAssembly bridge between the engine and a web page
web/                  the web demo (Vite + TypeScript)
layouts/              layout files
docs/DESIGN.md        design, research notes, roadmap
```

## License

MIT
