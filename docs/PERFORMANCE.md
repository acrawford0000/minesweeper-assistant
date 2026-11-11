# Performance and continuing boards

## Exact reuse

After logical propagation, only unknown cells touching unresolved clue constraints enter the frontier search. Each disconnected component has an exact histogram of arrangements by mine count, plus each cell's mine histogram. A bounded in-memory cache retains up to 32 complete component results, keyed by the precise cell sets and targets of its reduced constraints. A changed clue, flag, or membership changes the key. Incomplete or budget-exhausted enumeration is never cached. Global mine totals are applied afresh, so a counter edit can reuse a component histogram safely.

Components with identical per-cell histograms reuse the same global probability calculation. The cells outside the clue frontier form a single combinatorial pool. They already shared the mathematical numerator; the new implementation also computes the fraction and decimal formatting once, then copies the result. Binomial coefficients stop at the relevant remaining mine count, and the second weight vector derives from the first. All fractions remain arbitrary-precision exact counts.

The total probability and the global weights must still be recomputed after progress. In particular, an untouched right-hand area may gain or lose mine probability when clues on the left change. Similar rounded values such as 22% do not justify combining arbitrary constrained cells. Pooling uses actual constraint structure, not rounded display values.

The bottom stats retain elapsed milliseconds, frontier groups, and actual newly searched nodes. They also report reused components and pooled cells when applicable. A cached calculation can legitimately show zero search nodes while still containing frontier groups.

## Screenshot comparison

Before replacing the current reconstructed board, Rust compares dimensions and each cell's semantic state. At least three preserved revealed clues or flags, no changed known clues/flags, and further progress produce a **Continuing board** hint. Identical cell arrays produce **Matching board**. Sparse evidence is uncertain; conflicting known cells or dimensions indicate a changed position. The comparison is a convenience hint, not proof of game identity or an image-registration algorithm. No old clues or flags are merged automatically.

The cache independently requires exact constraint equality. Its validity does not depend on the screenshot hint, so even a mistaken similarity inference cannot justify reusing a different mathematical problem.

## Interface work

The heading, notifications, review controls, and editor no longer consume a large portion of the center. The board viewport measured 709 pixels tall in a 900-pixel-tall window at both 900 and 1320 pixel widths. Warnings are fixed-position toasts and do not shift the grid. Required review actions live in the right sidebar. The editor stays on the right at all supported desktop widths, and solver stats remain underneath the board.

Percentage elements remain mounted during recalculation. One shared ResizeObserver batches measurement reads before style writes; cells with the same text and board font size share a width measurement. This avoids thousands of independent observers and repeated layout flushes. A short input debounce remains, reduced to 80 ms, with revision checks protecting against stale results.

## Local checks

Representative release-engine measurements on this PC, excluding frontend drawing:

| Position | Before | After |
| --- | --- | --- |
| 50 × 50 covered board, 550 mines | 38 ms | 2 ms |
| Supplied early 48 × 33 board | 5–7 ms | 1 ms |
| Supplied dense board, repeated unchanged | 5,289 search nodes | 0 new nodes, 3 cached components |

These are local samples, not universal timing guarantees. Hard new frontiers still have the existing search/time budget, and rendering/IPC add to the visible update time.

37 Rust tests passed, including small-board full enumeration cross-checks, precise cache invalidation, incomplete-result exclusion, a 2,500-cell shared pool, and continuing-board comparison. Browser tests use the actual Rust engine to verify that cached results after a changed mine total exactly match a fresh process; the untouched pool's odds change appropriately. They also cover successive real screenshots, 900/1320/1920 layouts, toast stability, settings, zoom preservation, efficiency actions, and risk styles. A zoom on the 1,584-cell screenshot used one observer and 29 percentage width reads.
