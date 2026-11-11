# Screenshot regression fixtures

The classic, modern-light, and modern-dark images are synthetic boards with known state data.

The dark-classic-user fixture is the initial screenshot supplied for the flag and open-top-4 fixes. Its board state was transcribed and checked against the image.

The dark-classic-progress fixture is a subsequent real screenshot of the same game. Its labels use a separate fixture-labeling process based on the game's known solid RGB clue palette, white flag bases, and blank backgrounds. That process reproduces the earlier transcribed fixture exactly. The new image's 320 flags plus remaining-flags counter 42 retain the original 362-mine total. These fixed labels are test inputs; the production scanner is not given these cell positions or the expected board state.

The production recognizer instead searches geometry, inspects backgrounds, matches shapes, and derives displayed colors. These cases are regression coverage, not a representative held-out theme benchmark.
