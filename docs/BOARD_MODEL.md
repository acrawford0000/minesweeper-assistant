# Minesweeper Internal Board Model

The solver operates on semantic board data, independent of the screenshot theme. Recognition observations, user edits, and solver conclusions have separate records so corrections and recalculation remain auditable.

## Session and geometry

Store a schema version, session ID, monotonically increasing board revision, rows, columns, and the standard eight-neighbor rule. Cell IDs derive from row and column. Store image dimensions, crop bounds, lattice transform, geometry confidence, and cell-to-image coordinates separately. Custom dimensions are supported; practical memory/rendering limits must be shown explicitly.

## Cell states

| State | Meaning for analysis |
| --- | --- |
| Covered | Unknown mine variable |
| Revealed clue 0–8 | Known safe; imposes an adjacent-mine constraint |
| Flag | Fixed mine, matching the user's minesweeper.online play |
| Legacy confirmed mine | Migrated to the same flag state when loading older sessions |
| Safe without clue | Fixed safe, but supplies no adjacent-mine clue |
| Unreadable observation | Requires classification review before normal solving |
| Exploded mine or loss indicator | Ended-game observation; requires a distinct review mode |

For each cell retain the original observed label, alternate labels, calibrated confidence, source image crop, current accepted state, and whether a user overrode it. Solver-proven mines live in the derived analysis, not silently in the editable input. They must be recomputed if the board changes.

Do not interpret mines shown on a loss screen as ordinary pre-loss information. Postgame review needs explicit handling of wrong flags, revealed mines, and the original move position.

## Mine-count semantics

Use one canonical value: total mines in the board, or unknown. Every placed flag is a confirmed mine. The old trust-flags field is retained only for storage compatibility and does not change solving.

Offer two explicit entry modes:

- **Total mines:** input the board's total directly.
- **Game counter:** for a selected profile whose counter equals total minus placed flags, derive total = displayed counter + visually placed flags. Negative counter readings are allowed. Confirm the profile semantics and recognized flag count.

The counter is remaining flags, not the board's original mine total. On scan acceptance, the Rust engine computes total = detected counter + reviewed placed flags. Fixed flagged mines are then subtracted exactly once during solving. The UI shows the detected counter and lets the user opt out before acceptance. Later flag edits preserve the canonical total instead of reapplying an old screenshot counter.

Also allow direct editing of **mines unresolved**, labeled as excluding confirmed mines; convert that value back to the canonical total. Solver deductions alone must not silently change the user's total.

Unknown total is a valid state. The application still makes logical deductions, reports feasible mine-count ranges, and asks for a total before presenting a single unconditional set of global percentages. A density-based assumption can be offered later, visibly labeled.

## Analysis result

Each result carries board revision, assumptions, status, exact/estimated/uncomputed probabilities, proof traces, contradictions, timing, and cancelled/timed-out state. Exact fractions use integer numerator/denominator; display percentages are derived. A rounded 0% is never a proof of safety.

When no compatible configurations exist, show conflicting clues/counts and suppress move recommendations. When edits arrive during a calculation, mark old advice stale immediately and discard late results with the wrong revision.

## Editing and persistence

Represent edits as reversible commands: set cell, batch paint, update count, accept scan corrections, and import screenshot changes. Save semantic state, user overrides, and assumptions in local JSON. Images may be included optionally because they increase session size. Treat the analysis cache as disposable and validate imported files.
