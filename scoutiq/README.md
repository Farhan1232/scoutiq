# ScoutIQ

Football Manager 26 save-file assistant — Rust + Tauri + React + TypeScript.

This README covers **building and validating the FM26 save parser**: the
`Human Manager → Managed Club → Current Primary Competition` resolution path and
its Tauri/frontend integration.

## Build

```bash
# Frontend
npm install
npm run build            # tsc && vite build

# Rust library and desktop binary
cd src-tauri
cargo build --release

# Desktop bundle
npm run tauri build
```

## Research and validation binaries

`fm_probe` and `fm_catalogue` are **QA tools, not part of the shipped app**. They
sit behind the `research-tools` feature so `tauri build` cannot mistake one of
them for the application binary — without the gate the packaged `.deb` shipped
`fm_probe` in `/usr/bin` instead of ScoutIQ.

Because of that gate, every command that touches them needs the feature flag:

```bash
cargo build --release --features research-tools --bin fm_probe
cargo build --release --features research-tools --bin fm_catalogue
```

`cargo build --release --bin fm_probe` on its own fails by design, with
`target 'fm_probe' requires the features: 'research-tools'`.

### fm_probe

Read-only. Resolves one or more saves and prints what the parser recovers.

```bash
P=./target/release/fm_probe

$P --deep <save.fm>          # directory-driven resolver + game_db name table
$P --identity <save.fm>      # the exact path the Tauri command uses
$P --identity --json <s.fm>  # the full result payload, for diffing
$P --sections <save.fm>      # section directory listing
$P --clubrecords <save.fm>   # every club record answering to the club's name
$P --names 200 <save.fm>     # name-table records below an id
```

`--clubrecords` is how a club uid resolved without an inline hint is checked for
ambiguity: it prints each candidate record and whether they agree.

## Validation

```bash
cd src-tauri

# Unit tests. Live save checks skip loudly without SCOUTIQ_TEST_SAVES.
cargo test --release --lib

# The full suite, including read-only safety and the eight calibration saves.
SCOUTIQ_TEST_SAVES=/path/to/saves cargo test --release --lib

cargo build --release --features research-tools --bin fm_probe
cargo fmt --check
```

Frontend:

```bash
npx tsc --noEmit
npm run build
```

`SCOUTIQ_TEST_SAVES` points at a directory of `.fm` files. Without it the live
checks print `SKIPPED …` rather than passing silently — they were once keyed on
absolute paths from one machine and passed vacuously everywhere else.

### Read-only guarantee

Saves are opened with `File::open` only and are never written, renamed or
truncated. `read_only_safety::parsing_a_save_leaves_the_file_untouched` asserts
the property rather than the intent: it records size, modification time and
SHA-256 for every save, runs the full identity parser, and re-checks all three.

## Competition-name catalogue

`src-tauri/resources/competition_names/fm26.json` maps `competition uid →
default display name`. It only ever *labels* a uid; it holds no club data and
cannot influence which league a club is in. See that directory's README for
regeneration and for the known uid 22 gap.

The packaged app locates it through Tauri's resource directory and exports
`SCOUTIQ_COMPETITION_NAMES` at startup; set that variable yourself to override.

## Recommended IDE setup

[VS Code](https://code.visualstudio.com/) +
[Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) +
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
