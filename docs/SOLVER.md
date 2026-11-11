# Minesweeper Solver Algorithms

Use deterministic constraint propagation followed by exact model counting. These are the primary algorithms; machine learning belongs in image recognition, not in declaring a mathematically forced move.

## Constraints and deductions

Assign each unresolved covered cell a binary variable x: mine = 1, safe = 0. For every revealed clue c, require the sum of adjacent unresolved variables to equal c minus adjacent fixed mines. Include known-safe cells as zero. Every flag is a fixed mine, matching the requested minesweeper.online rules.

Validate residual counts against the number of adjacent variables. Repeatedly apply:

- Residual zero: all remaining adjacent variables are safe.
- Residual equal to variable count: all are mines.
- Subset subtraction: if constraint A's cells are a subset of B's, subtract A from B to derive a constraint on the difference.

Normalize/deduplicate derived constraints and bound their growth. Store which clues and assumptions prove each deduction. Report contradictions instead of continuing with invalid odds.

## Exact probabilities

The frontier consists of unresolved cells touching revealed clues. Build a bipartite clue/cell graph and split it into independent connected components. Enumerate valid assignments per component with depth-first backtracking, early lower/upper-bound pruning, constrained-variable ordering, and propagation after each branch.

For each component j, count valid assignments using k mines as A_j[k]. Also count assignments in which each cell v is a mine as B_j,v[k]. Use arbitrary-precision integers; exact counts can exceed ordinary machine integers.

Combine histograms through dynamic programming and convolution. Let R be mines remaining after fixed mines, and U be the number of unresolved cells outside the frontier. If the combined frontier consumes K mines, its completions have weight C(U, R−K), with impossible binomial arguments treated as zero.

The total number of compatible boards is:

`Z = sum_K combinedFrontierCount[K] * C(U, R-K)`

For a frontier cell, replace its component's histogram with that cell's mine histogram, combine the other components, apply the same weights, and divide by Z. For each off-frontier cell when U > 0, use C(U−1, R−K−1) in the numerator. Handle U = 0 separately.

This counts full-board completions rather than treating all frontier assignments as equally likely. Global weighting matters even when components do not touch. A worked treatment appears in this [probability calculator's mathematical explanation](https://cornsnek.github.io/minesweeper_probability_calculator/).

If Z = 0, the board is inconsistent. If a cell numerator is exactly zero or exactly Z, report proven safe or proven mine. Never infer certainty by rounding a percentage.

## Probability assumptions

Exact percentages are conditional on the accepted clues, declared mine total, fixed-mine assumptions, and a uniform distribution over compatible mine placements. A no-guess generator, seed-specific generator, or special first-click algorithm may produce a different prior. Explain the baseline and avoid claiming generator-aware odds unless that generator is modeled.

With an unknown mine total, retain sound logical deductions. Do not average across possible totals without specifying a prior. Optionally show the range of probabilities over feasible totals; label a chosen total as a scenario.

## Large or difficult frontiers

Runtime depends heavily on connected constraint structure, not just board dimensions. Use cancellation, node/time budgets, progress reports, and a responsive background task. Cache component histograms by normalized constraints; a mine-total change often needs only the final combination step.

The initial release should return sound deductions plus **not yet computed** on search timeout. Independently counted components can still require unknown weights from an unfinished component, so do not label their global percentages exact prematurely. Never interpret the fraction of a depth-first search visited so far as a probability estimate.

Later, evaluate equivalent-cell grouping, separator decomposition, and validated approximate model counting or importance sampling. Estimates require bias/convergence diagnostics, an uncertainty display appropriate to the method, and explicit estimated status. A sample frequency of zero does not prove safety. SAT can establish feasibility and forced cells, but an ordinary SAT solver alone does not count models or produce calibrated odds.

## Recommending moves

First show proven safe moves and proven mines. If no safe move exists, rank fully computed guesses by smallest mine probability and show ties. Do not call the best computed cell globally safest when other cells remain uncomputed.

Offer progress/information tie-breakers later: probability of a zero reveal, expected reduction in compatible boards, or bounded lookahead. Immediate survival and eventual win probability are different objectives. Only small enough endgames justify exact move-tree search. David Hill's solver documents probability inference, guessing heuristics, and endgame search as separate techniques. [JSMinesweeper algorithms](https://github.com/DavidNHill/JSMinesweeper#how-the-solver-determines-the-best-play)

## Correctness tests

Compare every exact count and fraction with a separately implemented full-board enumerator on tiny boards. Include off-frontier cells, multiple components, no frontier, equal-risk ties, global-count-induced certainties, wrong flags, safe-without-clue cells, missing totals, impossible boards, zero/all-mines edge cases, and cancellation. Benchmark representative expert/custom positions and adversarial large connected frontiers before setting performance promises.

