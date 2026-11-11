# Minesweeper Assistant Todo

## Version 0.5.1 single-key capture controls

- [x] Default the capture binding to Y and support other single letters/digits.
- [x] Start paused so launching the app does not intercept typing.
- [x] Add a visible on/off switch in the capture panel and Enable/Disable in Settings.
- [x] Retain the configured binding and capture area when disabled; re-enable without rebinding.
- [x] Keep paused bindings unregistered from Windows, including while editing them.
- [x] Confirm hotkey refresh never opens area selection and the manual button works while paused.

## Version 0.5.0 capture and clear recommendations

- [x] Add a full-desktop area selector with Enter/Escape, negative-origin and physical-DPI coordinate handling.
- [x] Capture and scan the selected area with a button or customizable global shortcut.
- [x] Exclude the system cursor and restore the assistant without raising it over the game.
- [x] Keep area/hotkey configuration in memory, with conflict handling and disable support.
- [x] Highlight the primary recommendation in bright blue and add a scroll-to-target button.
- [x] Preserve scan review, screenshot comparison, solver reuse, and zoom/pan behavior.
- [x] Verify native capture commands and shortcut registration alongside browser workflow tests.
- [ ] Consider an optional continuous, change-triggered capture mode later.

## Version 0.4.0 board focus and reuse

- [x] Compact the board header and move success/error messages to non-shifting toasts.
- [x] Move the editor and scan review to the right sidebar, preserving bottom solver stats.
- [x] Keep the board and both sidebars visible at supported desktop widths.
- [x] Compare successive reconstructed boards and identify likely progress without merging stale clues.
- [x] Cache complete exact frontier histograms by their actual constraints.
- [x] Compute and format outside-frontier odds once for the shared pool, with fresh global weights.
- [x] Batch percentage fitting with one observer and reuse percentage elements during updates.
- [x] Check cached results against fresh results after mine-total changes and preserve zoom.

## Version 0.3.2 view preservation

- [x] Preserve Fit and custom zoom through clue, flag, and mine-count updates.
- [x] Verify zoom and pan remain unchanged through recalculation in both modes.
- [x] Initialize the view only on initial load or a real change in board dimensions.

## Version 0.3.1 customization and stable editing

- [x] Add a Settings button with multiple keyboard bindings per editing/navigation action.
- [x] Capture keys, explain conflicts, support removing bindings and restoring defaults.
- [x] Add an independent board font slider and keep enlarged percentages within cells.
- [x] Keep customization in memory only; reset it at next launch without a settings file.
- [x] Keep the board stationary when warnings or notices appear or clear.
- [ ] Consider custom application themes later, without implementing them yet.

## Version 0.3.0 efficiency

- [x] Count flags, reveals and chords; compare direct reveals against flag/chord setups.
- [x] Charge shared flags once and include proven zero openings.
- [x] Bound exact current-frontier planning and label larger best-found plans honestly.
- [x] Add a persistent mode toggle, numbered board actions and sequence tracking.
- [x] Replan from the next screenshot without inventing hidden clues.
- [x] Automatically accept confident, readable scans after local consistency checks.
- [ ] Explore future-clue lookahead and expected whole-game click cost.

## Completed planning

- [x] Establish a separate project outline and default to Tauri with Svelte.
- [x] Define screenshot import, automatic dimensions, and uncertain-cell review.
- [x] Define board states, flag semantics, and mine-count editing.
- [x] Specify exact inference, global probability weighting, and timeout behavior.
- [x] Outline dark-mode interactions, move recommendations, and screenshot updates.
- [x] Define implementation stages and validation gates.

## Stage 1 Editable board and model

- [x] Scaffold the application and dark-mode shell.
- [x] Implement the versioned board model and eight-neighbor topology.
- [x] Add an arbitrary rows/columns board editor with validation and rendering limits.
- [ ] Support covered cells, numbers 0–8, flags (always confirmed mines), question marks, safe cells with unknown clues, and unreadable cells.
- [ ] Add keyboard entry, selection, undo/redo, batch editing, pan/zoom, and coordinate labels.
- [x] Add explicit total-mines and screenshot-counter controls without double subtraction.
- [x] Save/load local sessions and export a board as JSON/text.
- [x] Gate: edits round-trip without losing cell provenance or changing unrelated cells.

## Stage 2 Solver correctness

- [x] Validate clue bounds, duplicate constraints, impossible mine totals, and contradictions.
- [x] Implement propagation and subset subtraction with explanation traces.
- [x] Split the frontier into constraint-connected components.
- [x] Implement pruned enumeration with per-mine-count histograms and arbitrary-precision counts.
- [x] Combine components and unconstrained cells using the global mine total.
- [x] Return exact mine probabilities, safe moves, and confirmed mines separately.
- [x] Handle unknown totals with logic-only results.
- [ ] Add explicit feasible-total and probability ranges.
- [x] Treat flags as confirmed mines throughout recognition, solving, counter conversion, and session loading.
- [x] Add background execution, cancellation, board revisions, and search budgets.
- [x] Cache component counts across edits and mine-total changes.
- [x] Test against an independent exhaustive solver on tiny boards, including randomly generated valid positions.
- [x] Gate: identical configuration counts and per-cell fractions across the reference suite; no false certainty on timed-out searches.

## Stage 3 Screenshot geometry

- [ ] Assemble a consented or otherwise usable labeled screenshot corpus and track its provenance.
- [ ] Cover Windows classic/modern, minesweeper.online light/dark, unrelated themes, browser zoom, DPI scaling, compression, custom dimensions, and partial captures.
- [ ] Split evaluation by source theme/version and original image before augmentation.
- [x] Implement paste, file-open, drag/drop, image decode, and size limits.
- [ ] Add EXIF orientation normalization for photographed or rotated images.
- [ ] Detect candidate board rectangles, repeated tile spacing, grid origin, rows, and columns.
- [ ] Support multiple boards, hidden grid lines, irregular image scaling, and small rotation.
- [x] Add drag-to-crop and manual rows/columns as recognition fallbacks.
- [ ] Add direct pixel grid origin/spacing controls and resizable crop handles.
- [ ] Gate: publish exact-dimension accuracy and crop error on held-out images; do not silently accept a partial board as complete.

## Stage 4 Cell recognition

- [ ] Implement normalized tile crops, known-style templates, and a theme-independent cell classifier.
- [ ] Evaluate a small ONNX cell model using shape, border, and color evidence.
- [ ] Recognize covered, blank/zero, 1–8, flags, question marks, mines, explosion, wrong flags, and unreadable cells.
- [x] Recognize top-left red seven-segment remaining-flag counters and review the derived total.
- [ ] Support other counter fonts and calibrated counter confidence.
- [ ] Calibrate confidence thresholds on held-out themes and route ambiguous cells to review.
- [ ] Add a contradiction review that highlights possible misreads without silently rewriting clues.
- [ ] Gate: measure whole-board accuracy, automatic-accept accuracy, review rate, and rejection coverage by style; provisional goal is at least 99.5% correct automatically accepted boards on the agreed test set.
- [ ] Keep that goal distinct from a claim of universal screenshot support; record actual results before shipping.

## Stage 5 Complete companion workflow

- [x] Integrate the source image, editable board, recognition review, and move panel.
- [ ] Present mine-risk percentages, exact/estimated status, proof explanations, and ties.
- [x] Provide direct cell selection and number entry for results observed in the real game.
- [ ] Align a new screenshot to the current board; show changes and preserve manual corrections.
- [ ] Detect likely new games and ask whether to replace the session when alignment is ambiguous.
- [x] Recalculate after cell/mine-count changes; suppress stale recommendations immediately.
- [ ] Add autosave/recovery, image export, and accessibility checks.
- [ ] Gate: import → correct → recommend → play externally → update → recalculate works end to end on real screenshots.

## Later improvements

- [ ] Benchmark equivalent-cell grouping and component separators for larger searches.
- [ ] Add validated approximate counting/sampling with uncertainty diagnostics.
- [ ] Add expected-information and short-lookahead move ranking as optional strategies.
- [ ] Add saved recognition profiles, user-labeled tile examples, and recognition diagnostics.
- [ ] Add optional screenshot watching only as a separate user-enabled feature.
- [ ] Consider alternative rules/topologies only after the standard rectangular workflow is reliable.

## Current verification and next task

The Svelte workspace, Rust exact solver, and initial Rust grid/glyph scanner are implemented. Scanner regression tests cover synthetic classic, light, and dark boards; these are not a representative theme benchmark. Browser integration checks exercise the real Rust engine. Next, collect varied real screenshots, improve recognition and flag handling, and implement screenshot alignment/merging.


## Version 0.2.1 regression coverage

- [x] Fix flag and 4 recognition on the supplied dark classic screenshot.
- [x] Verify all 1,584 cells, 27 flags, seven 4s, and counter 335 gives total 362.
- [x] Remove the flag-trust toggle and separate confirmed-mine tool.


## Version 0.2.2

- [x] Add the nearly completed screenshot as a full-board regression case.
- [x] Compare multiple contrast passes and refine lattice phase to reject number baselines.
- [x] Use covered/revealed backgrounds to reject incompatible symbol readings.
- [x] Remove reliable blank cells from individual review highlighting.
- [x] Match displayed clue colors to the imported screenshot.
- [x] Add pointer-anchored wheel zoom and Ctrl-/middle-drag panning.
- [ ] Benchmark broader themes and fractional scaling before claiming universal client support.


## Version 0.2.3

- [x] Move the risk gradient to larger, whole-number percentage text.
- [x] Mute common middle groups and highlight tied extrema only.
- [x] Preserve the distinction between rounded endpoints and proven certainty.

