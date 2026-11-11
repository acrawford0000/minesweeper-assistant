# Efficiency planning

The goal is fewer total clicks to clear the board. Each flag, reveal, and chord costs one action. Flags are guaranteed mines in this app; the planner never invents a speculative flag to enable a chord. Clearing all safe cells completes the game, so unnecessary final flags are omitted.

## Current algorithm (Rust)

The probability solver first identifies proven safe cells and mines. The planner builds candidates for direct safe reveals, guaranteed zero openings with flood expansion, and chords on visible numbers whose remaining neighbors are all proved safe or mined. A chord candidate includes its required new flags. Shared flags are charged only once across the selected plan.

It minimizes the cost of covering the currently proven safe cells. For at most 24 safe cells, 20 distinct required flags, and 96 candidates, a memoized set-cover search runs for up to 100,000 nodes / 200 ms. A completed search is labeled minimum for current known work. Larger or interrupted searches return a best-found plan, starting from a greedy baseline with shared-flag pair lookahead. Ties favor fewer flags.

The displayed sequence is a prefix: any flags needed for the chosen action, then one reveal or chord. The next screenshot supplies its actual outcome before replanning. This allows discoveries and flood openings to change the plan, without fabricating hidden clues. Unknown future clues and potential chords on numbers not yet revealed are outside the current optimization. Therefore even a completed exact search is not a guarantee of the globally fewest clicks for the whole unseen board.

If no safe move exists, an available lowest-risk computed reveal is clearly labeled a guess. It makes no efficiency guarantee. Incomplete probability analysis can leave no recommendation.

## Workflow

Enable Efficiency mode in the header. Numbered board highlights match the action list: amber flags, blue chords, green safe reveals, and amber guesses. Percentages are hidden. Perform the actions in the game; the completion buttons only track the sequence. Paste the next screenshot after the reveal/chord. Screenshots scan automatically in this mode, and clear scans proceed without an acceptance click.

Scan acceptance requires every cell to be readable and above the current ambiguity threshold, compatible local clues/flags, and a feasible recognized counter. Otherwise review remains required. These checks reduce unnecessary review; recognition confidence is not a correctness guarantee. Solver contradictions still block a plan.

## Validation

Rust tests cover flag overhead, equal-cost no-flag preference, shared flags in both exact and large-frontier planning, zero openings, ordered flag/chord sequences, real-board action legality, guesses, and finished boards. Browser integration uses the actual Rust engine to exercise mode persistence, numbered actions, tracking without board mutation, automatic screenshot replacement, counter application, guesses, and restoring the probability overlay.
