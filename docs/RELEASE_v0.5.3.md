# Minesweeper Assistant v0.5.3

- Improved automatic board boundaries and row/column counts. Toolbar and frame lines no longer count as cell rows without perpendicular grid evidence.
- Improved spacing detection at fractional browser/display scales.
- Refine pixel alignment using complete cells and avoid rewarding false bevel readings on flat themes.
- Recover missing outer strips when actual cell dividers show the grid continues.
- Preserve genuine blank and unreadable edge rows. Low recognition confidence alone never removes real cells.
- Detect maximum-size custom boards even when an initial geometry candidate includes an extra toolbar row.
- Score each geometry candidate once to avoid repeated recognition work.

Download **Minesweeper-Assistant.exe** for the portable app or **Minesweeper-Assistant-Setup.exe** for the installer. **SHA256SUMS.txt** contains checksums.

Validation: 50 Rust tests, 8 frontend tests, frontend checks, and browser workflows with the real Rust scanner. Geometry tests cover headers, side edges, crops, margins, multiple themes, scaling, and custom boards. The supplied 32 × 20 screenshot is tested against all 640 cell labels and counter 140. Recognition remains local; unfamiliar or obstructed boards may still require a crop or correction.
