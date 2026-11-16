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

For the web demo, run these from `web/`:

```bash
pnpm wasm        # compile crates/intent-wasm into web/src/wasm (needs wasm-pack)
pnpm lint
pnpm typecheck
pnpm test        # needs `pnpm wasm` first: one suite runs the real engine
pnpm dev         # http://localhost:5173
```

The Rust version is pinned in `rust-toolchain.toml`. Bump it on purpose, then
fix whatever the newer clippy reports.

## Structure

- `crates/intent-core`: the engine. Pure: no I/O, no clock, no `unsafe`, no
  platform code. Hosts pass in timestamps and act on returned `Effect`s. Keep
  it this way so it can compile to WebAssembly and to firmware.
- `crates/intent-cli`: the `intent` tool. All logic sits behind
  `intent_cli::run`, which takes its output streams as arguments so tests do
  not spawn processes.
- `crates/intent-wasm`: the WebAssembly bridge. `view.rs` holds all behaviour
  as plain Rust, tested natively. `lib.rs` is only the `wasm-bindgen` wrapper.
- `web/`: the demo, in plain TypeScript with no framework. `engine.ts` wraps
  the WebAssembly module, `decoder.ts` calls the AI provider, `app.ts` draws
  the page.
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
- **The JSON shape is a contract.** `crates/intent-wasm/src/view.rs` and
  `web/src/engine.ts` describe the same data. Change both together; a Rust
  test locks the shape.
- **The core never does networking.** `intent-core::prompt` builds decoder
  messages; hosts send them.
- **API keys are the user's own.** The demo keeps a key in the browser's
  storage and sends it only to the chosen provider. Never put a key in the
  repository or in a `VITE_` variable, which would ship it to every visitor.

## Conventions

- Test names describe behaviour in plain words.
- Error messages say what is wrong and what was expected, and layout errors
  name the family and member.
- Changing the v0 layout's counts means updating the guard tests in
  `crates/intent-core/tests/v0_layout.rs` and the CLI tests.
- Update `CHANGELOG.md` with every user-visible change.
