# Minesweeper Assistant v0.5.2

- Improved recognition of minesweeper.online's 6 and 7 in light and dark modes, including small tile sizes and confusion with 8.
- Accepted clues and flags are remembered on continuing boards. Unchanged approved tiles no longer ask for confirmation on every screenshot.
- Approval memory survives reopening and saved sessions. Older reviewed sessions can reuse their saved screenshot.
- New or changed uncertain cells still require review. Matching board dimensions alone never bypasses review.
- Precomputed template shape features reduce repeated recognition work.

Download **Minesweeper-Assistant.exe** for the portable app or **Minesweeper-Assistant-Setup.exe** for the installer. **SHA256SUMS.txt** contains download checksums.

Validation: 43 Rust tests, 8 frontend tests, frontend type checks, and a browser workflow with the actual Rust scan engine. Digit tests cover 6/7/8 at seven sizes in both themes. Recognition remains local; unfamiliar artwork may still need correction.
