# Minesweeper Image Recognition Plan

Use a staged pipeline: image → board candidates → cell lattice → tile crops → cell hypotheses → consistency checks → reviewed board. Keep geometry confidence, cell recognition confidence, and game probabilities separate.

## Input and normalization

Accept clipboard image data, PNG/JPEG/WebP files, and drag/drop. Preserve the original image and record every crop/scale transform. Handle image orientation and alpha; reject corrupt or oversized inputs cleanly. Screenshots should normally need only translation and scaling. Offer rotation or perspective correction for photographed displays as a later extension.

Generate grayscale, locally normalized, edge, and color representations. Keep RGB information because clue colors can help, but never require classic blue-1/green-2 colors. Detect and test light/dark interpretations instead of blindly inverting all screenshots. Adaptive and Otsu thresholding are candidate preprocessing tools, not a complete recognizer. [OpenCV thresholding documentation](https://docs.opencv.org/4.x/d7/d4d/tutorial_py_thresholding.html)

## Automatic board dimensions

1. Find candidate regions using rectangular contours, edge density, repeated tile shapes, and aligned clue/symbol centers.
2. Estimate independent horizontal and vertical spacing with projection profiles and autocorrelation. Use line segments where grid lines exist; do not depend on them for borderless themes. OpenCV provides Hough line detection for the line-based candidate path. [OpenCV Hough documentation](https://docs.opencv.org/4.x/d9/db0/tutorial_hough_lines.html)
3. Fit a repeated lattice robustly, testing integer rows/columns, grid phase, and cell spacing. Preserve noninteger image coordinates to avoid cumulative rounding drift from DPI scaling.
4. Score candidates by spacing consistency, full-board boundary evidence, tile appearance, and recognizer agreement. Penalize toolbars, counters, and unrelated page grids.
5. Show the leading candidate and automatically detected dimensions. If several boards or dimensions remain plausible, let the user select one. Provide crop handles and grid adjustments without requiring manual dimensions on successful scans.

A crop missing an outer row cannot always reveal that row's existence. Require full-board boundary confirmation or an explicit partial-capture workflow; never infer a standard difficulty just to complete the rectangle.

## Cell recognition

Crop each tile twice: the full cell for border/covered-state evidence and the inner region for glyphs. Normalize to a small common size without losing the native crop. Classify zero, 1–8, covered, flag, question mark, mine, explosion, incorrect flag, or unreadable.

Use a hybrid recognizer. Fast normalized templates help known styles; a compact convolutional classifier trained across many styles handles broader variation. Generic OCR is a fallback for digits/counters, not the primary tile classifier: it does not understand the difference between a blank open cell and a blank covered cell. OpenCV template matching supplies a known-style baseline. [OpenCV template matching documentation](https://docs.opencv.org/4.x/d4/dc6/tutorial_py_template_matching.html)

For a future trained model, use packaged ONNX running locally through a Rust runtime in a background task. The initial implementation uses Rust grid and glyph matching. Compare its size, latency, accuracy, and packaging against simpler methods before committing. No cloud vision service is required. Optional user-approved cloud support would be a separate feature.

Train with scale, compression, antialiasing, glyph/font, border, contrast, color, and light/dark variation. Real held-out themes are necessary; recoloring a single template does not demonstrate broad support. Keep rare clues 6–8 and covered-versus-zero confusion visible in the confusion matrix.

## Review and consistency

Keep the top label candidates and recognition confidence per cell. Calibrate thresholds using validation data rather than treating raw model scores as probabilities. Review low-confidence cells, classifier disagreements, anomalous tiles, and uncertain dimensions.

Constraint checks can reject impossible interpretations and suggest alternate labels. They cannot prove a scan is correct: multiple incorrectly read boards can still be mathematically consistent. Do not change a scanned 3 to a 2 without showing and accepting the correction. An unreadable cell is not a covered cell.

Initially block final board-wide advice until unreadable cells and geometry issues are resolved. A later advanced mode may analyze multiple scan hypotheses, but must not mix visual uncertainty into mine odds without an explicit joint model.

## Updated screenshots

Rescan geometry and align by cell position plus stable revealed clues; do not require the original image size or exact colors. Compare detected changes with the saved model. Offer a change preview when an old clue changes, a manual correction conflicts, a board dimension differs, or a different game is likely. Preserve the original images and edit history.

## Evaluation

Report exact rows/columns accuracy, per-class recognition, fully correct boards, accuracy among automatically accepted boards, review/rejection rate, and end-to-end latency by style and image quality. A high per-cell score can hide frequent board errors, so whole-board accuracy is the primary quality gate. Benchmark data and confidence thresholds must be fixed before evaluating the final held-out set.

