# Minesweeper Assistant Development

The frontend is Svelte 5 with TypeScript and SvelteKit. All screenshot recognition, board constraints, exact counting, and move ordering run in Rust. No Python or JavaScript solver runs in the installed app.

## Run on this computer

```powershell
powershell -ExecutionPolicy Bypass -File .\Start-Dev.ps1
```

The helper locates the Node/npm and Rust tools installed in the workspace during the proof of concept. Keep this directory in its current workspace when using the helper.

## Standard commands

With Node.js 24+, npm, Rust and the Windows Tauri prerequisites installed:

```sh
npm install
npm run tauri dev
```

Build:

```sh
npm run tauri build
```

The executable is `src-tauri/target/release/minesweeper-assistant.exe`. The installer is in `src-tauri/target/release/bundle/nsis/`.

## Verification

```sh
npm run check
npm test
cd src-tauri
cargo test --release --lib
```

Rust tests include an independent full-board exhaustive reference for small positions, exact probabilities, disconnected components, flag semantics, contradictions, cancellation, and screenshot fixtures. Scanner fixtures are synthetic images with known ground truth. They establish regression coverage, not accuracy across arbitrary real-world themes.

The `engine` example is a line-oriented test harness used to connect browser UI integration tests to the same Rust implementation. It is not required by the installed app.

Digit references are rasterized glyph shapes generated from locally installed fonts; font binaries are not bundled. The matcher is deliberately conservative about unfamiliar symbols. A trained and calibrated recognition model remains future work.

Tauri prerequisites: https://v2.tauri.app/start/prerequisites/

## Version 0.2.1

Flags always mean mines. Old separate mine markers migrate to flags on session load. The scanner recognizes pennant/pole/base flags and open-top 4s. Top-left red seven-segment remaining-flag counters are read before the original image is cropped; acceptance uses Rust to add reviewed flags once. The supplied 48 x 33 screenshot is a full-cell regression fixture (27 flags, seven 4s, counter 335, total 362).

