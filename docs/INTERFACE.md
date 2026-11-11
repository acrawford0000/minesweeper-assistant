# Minesweeper Dark Mode Interface

The primary view places the editable reconstructed board in the center, screenshot review on the left, and recommendations on the right. On narrow windows, use tabs for Board, Screenshot, and Moves. The application stays dark regardless of whether the uploaded game uses a light or dark theme.

## Layout

```text
 Paste image   Open image   Save session       Scan status
 Dimensions 30 × 16   Total mines 99   Flags 12   Undo  Redo
 ┌────────────────┬────────────────────────┬─────────────────┐
 │ Original image │ Editable board         │ Recommended move│
 │ Crop and zoom  │ Coordinates and zoom   │ Exact / estimated│
 │ Uncertain     │ Mine risk overlay      │ Safe moves       │
 │ cells to check│ Selected cell outline  │ Lowest risk guesses
 │                │                        │ Explanation     │
 └────────────────┴────────────────────────┴─────────────────┘
 Editing tool   Number 0–8   Covered   Flag   Safe   Analysis status
```

Start with a large paste/drop area and **Ctrl+V** hint. After import, display automatically detected rows/columns and a visible review badge. Use a row/column coordinate convention consistently, displayed to users as one-based coordinates.

## Dark palette

Proposed tokens: background #101419, surfaces #181F27, raised surface #222D38, text #E6EDF3, secondary text #A7B3C2, focus #78B7FF, safe #4DD6AF, mine #FF7585, uncertainty #F5C96B. Validate contrast before implementation. Color is always paired with text, symbols, outlines, or patterns.

Keep reconstructed clue colors readable on dark cells, including a legible replacement for classic dark blue. Distinguish covered and revealed blank cells through border/fill shape as well as color. Retain original screenshot colors in the source pane for accurate comparison.

## Board editing

Click a cell to inspect it and choose its state. Keyboard arrows move selection; 0–8 enter revealed clues; F toggles a flag, which always means a mine; H restores covered state; S marks safe without a known clue. Provide a visible toolbar and context menu so shortcuts are optional. Escape cancels, Ctrl+Z undoes, and Ctrl+Shift+Z redoes. Avoid accidental changes during pan/zoom.

When the user clears a square in the original game, the companion prompts for the actual revealed number. If the result is blank, enter 0; if several cells opened, paste an updated screenshot or batch-enter their observed states. Do not fabricate adjacent numbers or simulate an unknown flood fill.

Edits trigger a short debounce and recalculate automatically. Show **Recalculating** immediately and remove actionable stale highlights. Use a separate scratch scenario mode for hypothetical outcomes so experiments do not overwrite the observed game.

## Recommendations and percentages

Default overlay: **mine risk**, explicitly labeled, with a toggle to **safe chance**. Covered cells show percentages; proven safe cells show a check, proven mines show a mine icon, estimated values show an approximation mark, and unavailable values show a dash. Preserve clue readability.

The move card reads, for example, **R6 C12 — 4.2% mine risk — Exact under current assumptions**. Guaranteed moves include a short proof and highlighted contributing clues. Guesses state that no safe move was found, show equal-risk alternatives, and never imply a guaranteed win. If the analysis is incomplete, state that the displayed recommendation is provisional.

Hover/focus shows the exact fraction where available, assumptions, and original cell crop. Recognition confidence appears only in scan review, labeled **Scan confidence**, not on the risk legend. Provide an accessible move list and cell inspector instead of relying solely on the visual heatmap.

## Review and recovery

Select an uncertain cell to see an enlarged source crop and alternate labels. Support accepting a correction and moving to the next uncertain cell. Allow grid/crop adjustment when every cell is misaligned. Contradictions highlight the conflicting clues and assumptions, with undo and source comparison nearby.

Keep mine total and counter edits explicit. Autosave locally; show a new-game/merge choice when a pasted screenshot conflicts with the active session. Offer board-image export with coordinates and a probability legend.

