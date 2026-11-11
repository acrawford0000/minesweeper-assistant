# Minesweeper Assistant v0.5.1

Windows desktop companion for Minesweeper, built with Svelte and Rust.

## Download

- **Minesweeper Assistant.exe** — portable application; download and run.
- **Minesweeper Assistant Setup.exe** — Windows installer.
- **SHA256SUMS.txt** — checksums for both downloads.

## Features

- Paste or capture a board screenshot, detect its grid, and calculate exact mine probabilities when search completes.
- Edit clues, flags, covered cells, and safe cells; flags mean confirmed mines.
- Efficiency mode suggests numbered reveal, flag, and chord sequences.
- A bright blue target makes the primary probability recommendation easy to locate.
- Mouse-wheel zoom, panning, customizable keyboard bindings, and board font sizing.
- Screen-area capture with a button or a customizable global hotkey.
- Reuse unchanged frontier calculations and compare successive board scans.

## v0.5.1 changes

The global capture key defaults to **Y** and starts **paused**. Enable it from the left capture panel or Settings after choosing a capture area. Pause it before typing in other apps. Disable keeps the selected area and binding, and Enable restores the saved key. The manual capture button works while the hotkey is paused.

## Notes

Capture area and customization are kept for the current open app. Captures, scanning, and solving run locally. Recognition can need correction on unfamiliar themes; uncertain scans require review. Hard frontier searches are bounded and can return partial results. Blue identifies the recommended move, including guesses, and is not itself a guarantee of safety.

Validation: 40 Rust tests, frontend checks, native capture/shortcut smoke tests, and browser workflow checks passed for this build.
