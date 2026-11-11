# Minesweeper Assistant

A working Windows desktop beta with a **Svelte 5** interface and **Rust** algorithms. It replaces the random-number-generator proof of concept.

## Open the app

Download the portable **Minesweeper Assistant.exe** or **Minesweeper Assistant Setup.exe** installer from [GitHub Releases](https://github.com/acrawford0000/minesweeper-assistant/releases). The source is in [app](app), with build instructions in [the development guide](app/README.md).

For the original local workspace, both executables are also in the parent `outputs` folder.

## What works

- Dark-mode workspace with an editable board, move list, and square inspector.
- Paste, open, or drop PNG/JPEG/WebP screenshots; drag on the source image to crop.
- Initial Rust recognition of repeated grids and clue shapes, including automatic rows/columns. Manual grid controls and cell corrections provide a fallback.
- Clear, confident scans proceed straight to analysis. Unreadable, ambiguous, or locally inconsistent scans require review.
- Toggleable Efficiency mode compares reveals, zero openings, and flag/chord setups, including shared flags, to reduce total clicks.
- Rust logical deductions and exact global mine probabilities using component counting and arbitrary-precision integers.
- Custom dimensions, editable total mines, a game-counter helper, flags that are confirmed mines, and safe cells whose clue has not yet been entered.
- Automatic recalculation, bounded search, stale-result protection, undo/redo, session export/import, and local autosave.

## Quick start

1. Paste a screenshot with **Ctrl+V**, or choose **Open image**.
2. Choose **Scan screenshot**. Crop or use **Adjust grid** if the detected dimensions are wrong.
3. Clear scans are accepted automatically. If review is needed, correct cells using **0–8**, flag, covered, or safe, then choose **Accept scan**.
4. A recognized remaining-flag counter sets **Total mines** automatically. Otherwise enter the total for the entire board, or use the counter helper.
5. Make the recommended move in the actual game. Here, select its square and enter the revealed number. Odds update automatically.

A blank revealed tile is **0**. **S** means safe with its clue still unknown; it does not invent a zero or reveal its neighbors. Right-click or **F** toggles a flag (a confirmed mine). **H** restores a covered cell. Use **Fit**, zoom, and scrolling for large boards.

## Current limits

The recognizer is an initial grid/glyph matcher, not a universal trained vision model. It is verified on controlled classic/light/dark fixtures and has been exercised against a browser-rendered classic board. Arbitrary themes, flags, borderless boards, small images, overlays, and unusual fonts may require manual correction. Visual match scores are not calibrated confidence percentages.

Importing a new screenshot replaces the reconstructed board after scanning; automatic alignment and merging with earlier manual corrections remain planned. Undo retains the preceding position. Red seven-segment remaining-flag counters above the board are recognized. Other counter styles still use the manual helper. Loss-screen interpretation, advanced approximate counting, and hidden-clue lookahead are not implemented.

The solver gives exact odds when it completes, assuming a uniform distribution over mine placements compatible with the accepted clues and total. Hard frontiers stop at a search/time budget; uncomputed values stay blank. Logical deductions remain available without a mine total. Unknown custom game-generation rules can imply a different probability prior.

Autosave retains board data locally. Large source images can be omitted from browser storage; **Save session** includes the image. Screenshots and solving stay on the computer; no cloud service is used.

## Project documents

- [Todo list](TODO.md)
- [Image recognition plan](docs/IMAGE_RECOGNITION.md)
- [Solver algorithms](docs/SOLVER.md)
- [Board model](docs/BOARD_MODEL.md)
- [Interface plan](docs/INTERFACE.md)
- [Architecture](docs/ARCHITECTURE.md)

The design documents include later goals. See the todo list and the limits above for the current implementation boundary.

## Version 0.2.1 fixes

Flags always mean confirmed mines. The trust toggle and duplicate mine tool are removed. Recognition includes flag pennant/pole/base detection and an open-top 4 check. The supplied 48 x 33 dark classic screenshot has a full-cell regression test: 27 flags, seven 4s, and counter 335 (362 total mines). This verifies that image, not universal theme accuracy.


## Version 0.2.2

The scanner now compares multiple contrast passes and grid alignments, distinguishes cell backgrounds, and automatically handles reliable blanks. Clue colors are sampled from the screenshot. Mouse-wheel zoom is anchored at the pointer; Ctrl + left-drag and middle-drag pan without editing cells. Large boards start at a readable zoom rather than being squeezed into tiny cells.

See [Current screenshot recognition](docs/CURRENT_SCANNER.md) for the actual algorithm and remaining limits.


## Version 0.2.3

The risk overlay now uses larger, whole-number percentages. Saturated text colors carry the risk scale, common middle groups are gray, and only safest/highest-risk cells receive full green/red fills. Exact solver values and certainty are preserved. See [Risk display](docs/RISK_DISPLAY.md).

## Version 0.3.0

Turn on **Efficiency mode** to replace percentages with a numbered action sequence. Every flag, reveal, and chord costs one click. Flags are suggested only as part of a click-saving plan; direct reveals win equal-cost ties that would add flags. Follow the sequence in the game, then paste the next screenshot. In this mode, new screenshots automatically replace, scan, and replan; Undo retains the previous board. Marking a step done only tracks progress, without inventing revealed clues.

Small frontiers use bounded exact planning; larger ones use a budgeted heuristic, clearly labeled best-found. The objective covers currently proven safe squares using visible clues and proven zero openings. New information can change the best continuation, so this does not promise a globally minimum-click solution to unseen cells. Guesses are explicitly labeled. See [Efficiency planning](docs/EFFICIENCY.md).

## Version 0.3.1

**Settings** in the header lets you add or remove multiple keyboard bindings for each clue, flag, covered, safe, unreadable, movement, undo, and redo action. For example, add **Q** to **0 · Revealed blank** while retaining **0**. Conflicting assignments are explained; clipboard and system shortcuts remain reserved. Mouse controls stay fixed.

The board font slider changes clues, percentages, and symbols without changing cell dimensions, zoom, or other interface text. Enlarged percentages fit within their cells. Settings apply immediately, can be restored to defaults, and exist only for the current open app: no settings file or new preference storage is used. Board/session autosave continues independently. Custom themes remain a future possibility.

Warning and notice messages now occupy a fixed-height area. Incorrect clues, corrections, and dismissing notices leave the grid at the same screen coordinates, avoiding cascading misclicks.

## Version 0.3.2

Clue, flag, and mine-count edits preserve the current zoom and pan in both modes, including a view chosen with Fit. Automatic view setup runs only on initial load or when board dimensions change; routine recalculation no longer resets zoom.

## Version 0.4.0

The center is now almost entirely board: a compact toolbar replaces the large heading/status area, the cell editor and scan-review actions move to the right sidebar, and notifications appear as dismissible toasts. Calculation time, frontier groups, and search nodes remain at the bottom. This layout keeps both sidebars beside the board even in a 900-pixel-wide window.

Successive scans are compared by dimensions, revealed clues, and flags. A likely continuation shows **Continuing board**, with newly revealed and still-covered cell counts. This is a comparison of reconstructed board states, not proof of game identity; it never silently merges old clues into a screenshot.

Rust reuses complete counts for exactly matching frontier constraints in a bounded memory cache. The outside-frontier cells share one probability calculation and formatting step; binomial weights are computed only through the relevant mine count. Their odds are reweighted whenever current information changes, because a still-covered area can have different odds after progress elsewhere. Equal rounded percentages alone are never treated as equivalent constraints.

Rendering batches percentage sizing by label with one shared observer. The input delay before recalculation is reduced from 220 to 80 ms, and percentage elements are reused through updates. See [Performance and board comparison](docs/PERFORMANCE.md) for measurements and correctness checks. The caches live in memory and disappear when the app closes.

## Version 0.5.0

Use **Choose capture area** in the left sidebar, then drag around the game board and its remaining-mine counter. **Enter** or **Use this area** confirms; **Esc** cancels and preserves any previous area. The first selected image scans immediately. Thereafter use **Capture & analyze** or the global **Ctrl + Shift + Space** hotkey while playing. Change or disable the shortcut in **Settings → Screen capture hotkey**.

Capture uses a fixed rectangle in physical screen pixels, across monitors and DPI scales. Keep the game visible in that position and reselect if it moves or resizes. The assistant hides briefly so it cannot appear in its own capture, then restores its previous visibility, Z order, and focus intent. The system cursor is not drawn into the capture. Game hover effects and tooltips remain actual screen content. This is on-demand capture, without continuous streaming or automatic clicks in the game. The capture area and shortcut last for the current open app; no settings file is created.

The primary lowest-risk recommendation now has a **bright blue fill, cyan outline, and glow**, distinct from green safe cells and red mines. One target is marked even when several cells tie. Blue identifies the recommendation and does not by itself mean guaranteed safe. Use the toolbar crosshair or **Show blue target** to bring it into view without changing zoom. Efficiency mode retains its numbered action highlights.

See [Screen capture implementation and checks](docs/SCREEN_CAPTURE.md).

## Version 0.5.1

The global capture binding now defaults to the single key **Y**, and starts **paused** on each launch so opening the app does not intercept typing. Turn it on with **Global capture hotkey** in the left capture panel, or **Enable** in Settings. Y then captures and analyzes the existing selected area; it never starts area selection.

**Pause/Disable** genuinely unregisters the key from Windows. The binding and area remain remembered for the session, so **Enable** restores them. Changing the binding while paused keeps it paused. The capture button works regardless of the hotkey state. Use Pause before typing messages in another app; an enabled single-key global hotkey intercepts that key system-wide.

