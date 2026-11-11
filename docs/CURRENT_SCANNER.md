# Current screenshot recognition

All runtime recognition algorithms run in Rust. This is classical computer vision, not an OCR service or a trained neural network.

1. Decode the original screenshot locally and preserve it for the counter and color samples.
2. Measure horizontal and vertical edges at four contrast caps. Lower caps prevent bright number strokes from overpowering quieter grid borders.
3. Fit repeated lattices, refine their spacing and phase, and compare candidate cell crops and boundary evidence. The progressed screenshot exposed a five-pixel vertical alignment error in the older single-pass approach.
4. Examine full cell backgrounds and borders. Learn dominant covered and revealed backgrounds from the board itself where they differ.
5. Inspect the inner symbol. Flags use their colored pennant plus contrasting pole/base; clues use normalized glyph templates and shape features. A number on a covered background is rejected for review rather than accepted as a 7.
6. Confirm blank cells from background/border evidence. Reliably classified blanks are not included in per-cell review highlights.
7. Read the dominant solid glyph colors of accepted clues and send those colors to Svelte for rendering.
8. Read a red seven-segment remaining-flags counter above the board, then add reviewed placed flags exactly once when the scan is accepted.

No cloud service receives the screenshot. Colors, shapes, and background evidence complement each other; color alone is not used to infer an arbitrary client's number meanings. The exact solver remains separate from recognition.

## Verification and limits

Regression coverage includes controlled classic/light/dark fixtures and two real 48 x 33 screenshots at different stages of the same game. The progressed board contains 320 flags, 66 fours, and counter 42, retaining the same total of 362 mines. Tests compare all 1,584 cells and the learned clue colors, not just selected examples.

This does not establish universal client accuracy. Borderless themes, unfamiliar symbols/fonts, unusual fractional scaling, partial boards, and overlays still need broader labeled datasets and evaluation. A trained and calibrated cell classifier remains a future option. Slow scanning by itself cannot restore missing image information or replace validation.
