# Risk display

Percentages are displayed as rounded whole numbers. The solver retains exact configuration counts and uses unrounded probabilities for recommendations and highlights.

- Text follows a saturated green → orange → red scale from 0% to 100% mine risk.
- Common middle bands of similar odds are gray. The Rust presentation module groups values in two-percentage-point bands and mutes sufficiently common groups, preserving emphasis on the extremes.
- Green full-cell fill marks proven safe cells and all tied lowest-risk covered cells.
- Red full-cell fill marks proven mines and all tied highest-risk covered cells.
- Other covered cells use the same neutral background; there is no intermediate heatmap shading.
- If every covered cell has identical non-certain odds, no cell is arbitrarily highlighted as the best or worst. A sufficiently large uniform group is gray.
- A rounded 0% or 100% is not automatically a certainty. Small ✓ / ⚑ markers distinguish proven results; hover text and the inspector preserve this distinction. Exact fractions remain available.

Mine-risk colors remain consistent when the optional safe-chance display is selected. Clue colors from the source screenshot are unchanged.
