use super::*;

pub(super) fn lattice(profile: &[f64]) -> Option<(usize, usize, usize)> {
    lattice_matching(profile, None)
}

fn lattice_matching(profile: &[f64], expected_step: Option<f64>) -> Option<(usize, usize, usize)> {
    let max = profile.iter().copied().fold(0.0, f64::max);
    if max < 1.0 {
        return None;
    }
    let mut peaks = Vec::new();
    for i in 1..profile.len().saturating_sub(1) {
        if profile[i] < max * 0.18
            || profile[i] < 1.5
            || profile[i] < profile[i - 1]
            || profile[i] < profile[i + 1]
        {
            continue;
        }
        if let Some(&last) = peaks.last() {
            if i - last <= 2 {
                if profile[i] > profile[last] {
                    peaks.pop();
                } else {
                    continue;
                }
            }
        }
        peaks.push(i);
    }
    let mut best = None;
    let mut best_score = 0.0;
    // Repeat spacing is inferred from actual edge positions, not difficulty presets.
    for &start in &peaks {
        let left = peaks.partition_point(|&p| p <= start + 5);
        let right = peaks.partition_point(|&p| p <= start + 130);
        for &next in &peaks[left..right] {
            let mut step = (next - start) as f64;
            if expected_step.is_some_and(|expected| (step / expected - 1.0).abs() > 0.15) {
                continue;
            }
            let initial_step = step;
            if let Some(expected) = expected_step {
                step = expected;
            }
            let mut count = 1;
            let mut end = next;
            let mut strength = profile[start] + profile[next];
            loop {
                let expected = start as f64 + (count + 1) as f64 * step;
                let lo = peaks.partition_point(|&p| (p as f64) < expected - 2.5);
                let hi = peaks.partition_point(|&p| (p as f64) <= expected + 2.5);
                let matched = peaks[lo..hi]
                    .iter()
                    .copied()
                    .filter(|&p| p > end)
                    .max_by(|&a, &b| profile[a].total_cmp(&profile[b]));
                if let Some(p) = matched {
                    count += 1;
                    end = p;
                    strength += profile[p];
                    // Browser/display scaling gives fractional tile pitches.
                    // Refit from the whole observed span instead of accumulating
                    // the rounding error of the first pair of pixel edges.
                    let observed = (end - start) as f64 / count as f64;
                    step =
                        (step * 0.8 + observed * 0.2).clamp(initial_step * 0.9, initial_step * 1.1);
                } else {
                    break;
                }
                if count >= 100 {
                    break;
                }
            }
            if count < 3 {
                continue;
            }
            let mut first = start;
            loop {
                let expected = first as f64 - step;
                if expected < 0.0 {
                    break;
                }
                let lo = peaks.partition_point(|&p| (p as f64) < expected - 2.5);
                let hi = peaks.partition_point(|&p| (p as f64) <= expected + 2.5);
                let matched = peaks[lo..hi]
                    .iter()
                    .copied()
                    .filter(|&p| p < first)
                    .max_by(|&a, &b| profile[a].total_cmp(&profile[b]));
                if let Some(p) = matched {
                    first = p;
                    count += 1;
                    strength += profile[p];
                } else {
                    break;
                }
                if count >= 100 {
                    break;
                }
            }
            let score = (count as f64).powf(1.3) * (strength / (count + 1) as f64) * step.sqrt();
            if score > best_score {
                best_score = score;
                best = Some((first, end, count));
            }
        }
    }
    best.map(|(mut first, mut end, mut count)| {
        // A tightly cropped screenshot may have no exterior transition at its
        // boundary. Extend one cell only when the lattice meets that boundary.
        let step = (end - first) as f64 / count as f64;
        if (first as f64 - step).abs() <= 2.5 {
            first = 0;
            count += 1;
        } else if first <= 2 {
            first = 0;
        }
        if ((end as f64 + step) - profile.len() as f64).abs() <= 2.5 {
            end = profile.len();
            count += 1;
        } else if profile.len().saturating_sub(end) <= 2 {
            end = profile.len();
        }
        (first, end, count)
    })
}

pub(super) fn detect(image: &RgbImage) -> Result<(Rect, usize, usize), String> {
    let w = image.width() as usize;
    let h = image.height() as usize;
    let caps = [48.0, 64.0, 96.0, 160.0];
    let mut xp = vec![vec![0.0; w]; caps.len()];
    let mut yp = vec![vec![0.0; h]; caps.len()];
    for y in 1..h {
        for x in 1..w {
            let p = &image.get_pixel(x as u32, y as u32).0;
            let dx = distance(p, &image.get_pixel(x as u32 - 1, y as u32).0);
            let dy = distance(p, &image.get_pixel(x as u32, y as u32 - 1).0);
            for (k, cap) in caps.iter().enumerate() {
                xp[k][x] += dx.min(*cap) / h as f64;
                yp[k][y] += dy.min(*cap) / w as f64;
            }
        }
    }
    let mut candidates: Vec<(Rect, usize, usize)> = Vec::new();
    for k in 0..caps.len() {
        let x_lattice = lattice(&xp[k]);
        let y_lattice = lattice_matching(
            &yp[k],
            x_lattice.map(|(x, right, cols)| (right - x) as f64 / cols as f64),
        );
        let (Some((x, right, cols)), Some((y, bottom, rows))) = (x_lattice, y_lattice) else {
            continue;
        };
        // Remove frame strips before enforcing the board-size limit: an extra
        // toolbar row must not make a valid 2,500-cell board disappear.
        if (right - x) / cols < 6 || (bottom - y) / rows < 6 {
            continue;
        }
        let candidate = (
            Rect {
                x: x as u32,
                y: y as u32,
                width: (right - x) as u32,
                height: (bottom - y) as u32,
            },
            rows,
            cols,
        );
        if !candidates.iter().any(|(r, rs, cs)| {
            r.x == candidate.0.x
                && r.y == candidate.0.y
                && r.width == candidate.0.width
                && r.height == candidate.0.height
                && *rs == rows
                && *cs == cols
        }) {
            candidates.push(candidate);
        }
    }
    let originals = candidates.clone();
    for (rect, rows, cols) in originals {
        let (x, width) = refine_axis(&xp[1], rect.x, rect.width, cols);
        let (y, height) = refine_axis(&yp[1], rect.y, rect.height, rows);
        let fitted = Rect {
            x,
            y,
            width,
            height,
        };
        if !candidates.iter().any(|(r, rs, cs)| {
            r.x == x
                && r.y == y
                && r.width == width
                && r.height == height
                && *rs == rows
                && *cs == cols
        }) {
            candidates.push((fitted, rows, cols));
        }
    }
    // A toolbar/frame edge can continue the one-dimensional lattice. Verify
    // actual perpendicular cell edges inside the outer bands before scoring.
    for candidate in &mut candidates {
        *candidate = extend_frame(image, trim_frame(image, *candidate));
    }
    // A divider often has a light edge and a dark edge a pixel apart. Picking
    // the strongest gradient is not the same as finding the tile's origin.
    // Compare nearby pixel phases using complete cell appearances.
    let mut phases: Vec<(Rect, usize, usize)> = Vec::new();
    let mut phase_offsets: Vec<u32> = Vec::new();
    for (rect, rows, cols) in candidates {
        for dy in -2i32..=2 {
            for dx in -2i32..=2 {
                let x = rect.x as i32 + dx;
                let y = rect.y as i32 + dy;
                if x < 0
                    || y < 0
                    || x as u32 + rect.width > image.width()
                    || y as u32 + rect.height > image.height()
                {
                    continue;
                }
                let existing = phases.iter().position(|&(r, rs, cs)| {
                    r.x == x as u32
                        && r.y == y as u32
                        && r.width == rect.width
                        && r.height == rect.height
                        && rs == rows
                        && cs == cols
                });
                let offset = dx.unsigned_abs() + dy.unsigned_abs();
                if let Some(i) = existing {
                    phase_offsets[i] = phase_offsets[i].min(offset);
                } else {
                    phases.push((
                        Rect {
                            x: x as u32,
                            y: y as u32,
                            ..rect
                        },
                        rows,
                        cols,
                    ));
                    phase_offsets.push(offset);
                }
            }
        }
    }
    // Strong glyph baselines can mimic a lattice on a nearly completed board.
    // Compare independent contrast passes by inspecting complete tile crops.
    let quality = |&(rect, rows, cols): &(Rect, usize, usize)| -> f64 {
        let stride = (rows * cols / 96).max(1);
        let mut score = 0.0;
        let mut count = 0;
        for i in (0..rows * cols).step_by(stride) {
            let tile = classify(image, cell_rect(rect, rows, cols, i));
            // Blank covered and revealed cells are distinguished later using
            // learned backgrounds. Rewarding a guessed bevel here favors a
            // misaligned crop on flat themes, even when its grid is worse.
            score += if tile.empty { 0.72 } else { tile.score }
                - if tile.state == -5 { 0.8 } else { 0.0 }
                - if tile.state > 0 && tile.bevel {
                    0.6
                } else {
                    0.0
                };
            count += 1;
        }
        let boundary_strength = |profile: &[f64], start: u32, length: u32, cells: usize| {
            let max = profile.iter().copied().fold(0.0, f64::max).max(1.0);
            (1..cells)
                .map(|i| {
                    let p = start as usize + i * length as usize / cells;
                    profile[p.saturating_sub(4)..(p + 5).min(profile.len())]
                        .iter()
                        .copied()
                        .fold(0.0, f64::max)
                        / max
                })
                .sum::<f64>()
                / (cells - 1).max(1) as f64
        };
        score / count as f64
            + 0.25
                * (boundary_strength(&xp[1], rect.x, rect.width, cols)
                    + boundary_strength(&yp[1], rect.y, rect.height, rows))
    };
    // Score each candidate once. The old comparator repeatedly classified both
    // boards while finding the maximum.
    phases.into_iter().zip(phase_offsets).filter(|((_,rows,cols),_)|*rows<=100 && *cols<=100 && rows*cols<=2500).map(|(c,offset)| (quality(&c)-0.002*offset as f64, c)).max_by(|a,b|a.0.total_cmp(&b.0)).map(|(_,c)|c).ok_or("I couldn't verify a reliable board grid. Crop the board and use Adjust grid if needed.".into())
}

fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_unstable_by(f64::total_cmp);
    values[values.len() / 2]
}

// For a proposed row, measure vertical cell dividers through its interior;
// for a column, measure horizontal dividers. A horizontal toolbar rule alone
// cannot provide this evidence. Median pooling tolerates glyphs and damage.
fn band_support(
    image: &RgbImage,
    rect: Rect,
    rows: usize,
    cols: usize,
    horizontal: bool,
) -> Vec<f64> {
    let (bands, dividers) = if horizontal {
        (rows, cols)
    } else {
        (cols, rows)
    };
    (0..bands)
        .map(|band| {
            let tile = cell_rect(
                rect,
                rows,
                cols,
                if horizontal { band * cols } else { band },
            );
            let (along, span, across, extent) = if horizontal {
                (tile.y, tile.height, rect.x, rect.width)
            } else {
                (tile.x, tile.width, rect.y, rect.height)
            };
            let radius = ((extent / dividers as u32) / 10).clamp(1, 3) as i32;
            let mut lines: Vec<f64> = (1..dividers)
                .map(|i| {
                    let boundary = across + i as u32 * extent / dividers as u32;
                    (-radius..=radius)
                        .map(|offset| {
                            let edge = boundary as i32 + offset;
                            if edge < 1
                                || (horizontal && edge >= image.width() as i32)
                                || (!horizontal && edge >= image.height() as i32)
                            {
                                return 0.0;
                            }
                            (0..8)
                                .map(|s| {
                                    let position =
                                        (along + span / 5 + (span * 3 / 5).max(1) * s / 8)
                                            .min(along + span - 1);
                                    let (x, y, px, py) = if horizontal {
                                        (edge as u32, position, edge as u32 - 1, position)
                                    } else {
                                        (position, edge as u32, position, edge as u32 - 1)
                                    };
                                    distance(&image.get_pixel(x, y).0, &image.get_pixel(px, py).0)
                                        .min(96.0)
                                })
                                .sum::<f64>()
                                / 8.0
                        })
                        .fold(0.0, f64::max)
                })
                .collect();
            median(&mut lines)
        })
        .collect()
}

fn trim_frame(image: &RgbImage, candidate: (Rect, usize, usize)) -> (Rect, usize, usize) {
    let (mut rect, mut rows, mut cols) = candidate;
    // Keep genuinely blank revealed bands, even on themes which hide dividers
    // between opened cells. A row of unreadable symbols with real dividers is
    // also retained: recognition confidence alone never removes board cells.
    let stride = (rows * cols / 160).max(1);
    let open_backgrounds: Vec<_> = (0..rows * cols)
        .step_by(stride)
        .filter_map(|i| {
            let t = classify(image, cell_rect(rect, rows, cols, i));
            (t.state > 0 && t.score >= 0.8 && !t.bevel).then_some(t.bg)
        })
        .collect();
    for horizontal in [true, false] {
        loop {
            let bands = if horizontal { rows } else { cols };
            if bands <= 2 {
                break;
            }
            let support = band_support(image, rect, rows, cols, horizontal);
            let typical = median(&mut support.clone());
            if typical < 4.0 {
                break;
            }
            let absent = |band: usize| {
                if support[band] >= typical * 0.16 {
                    return false;
                }
                let n = if horizontal { cols } else { rows };
                let blank_open = (0..n)
                    .filter(|&i| {
                        let index = if horizontal {
                            band * cols + i
                        } else {
                            i * cols + band
                        };
                        let t = classify(image, cell_rect(rect, rows, cols, index));
                        t.empty
                            && !t.bevel
                            && open_backgrounds.iter().any(|bg| distance(bg, &t.bg) < 12.0)
                    })
                    .count();
                blank_open * 2 < n
            };
            let first = absent(0);
            let last = absent(bands - 1);
            if !first && !last {
                break;
            }
            let step = if horizontal {
                rect.height as f64 / rows as f64
            } else {
                rect.width as f64 / cols as f64
            };
            let inset = step.round() as u32;
            if horizontal {
                if first {
                    rect.y += inset;
                    rect.height -= inset;
                    rows -= 1;
                }
                if last {
                    rect.height -= inset;
                    rows -= 1;
                }
            } else {
                if first {
                    rect.x += inset;
                    rect.width -= inset;
                    cols -= 1;
                }
                if last {
                    rect.width -= inset;
                    cols -= 1;
                }
            }
        }
    }
    (rect, rows, cols)
}

// A strong toolbar or glyph edge can also interrupt a valid lattice early.
// Grow it only where a full adjacent strip has the same perpendicular grid.
fn extend_frame(image: &RgbImage, candidate: (Rect, usize, usize)) -> (Rect, usize, usize) {
    let (mut rect, mut rows, mut cols) = candidate;
    for horizontal in [true, false] {
        loop {
            let (bands, other, origin, length, limit) = if horizontal {
                (rows, cols, rect.y, rect.height, image.height())
            } else {
                (cols, rows, rect.x, rect.width, image.width())
            };
            if bands >= 100 || (bands + 1) * other > 2500 {
                break;
            }
            let typical = median(&mut band_support(image, rect, rows, cols, horizontal));
            if typical < 4.0 {
                break;
            }
            let step = (length as f64 / bands as f64).round() as u32;
            let mut accepted = None;
            for first in [true, false] {
                let (new_origin, new_length) = if first {
                    if origin + 2 < step {
                        continue;
                    }
                    let start = origin.saturating_sub(step);
                    (start, length + origin - start)
                } else {
                    if origin + length + step > limit + 2 {
                        continue;
                    }
                    (origin, (length + step).min(limit - origin))
                };
                let mut next = rect;
                let (rs, cs) = if horizontal {
                    next.y = new_origin;
                    next.height = new_length;
                    (rows + 1, cols)
                } else {
                    next.x = new_origin;
                    next.width = new_length;
                    (rows, cols + 1)
                };
                let support = band_support(image, next, rs, cs, horizontal);
                let edge = if first {
                    support[0]
                } else {
                    *support.last().unwrap()
                };
                if edge >= (typical * 0.18).max(3.0) {
                    accepted = Some((next, rs, cs));
                    break;
                }
            }
            if let Some((next, rs, cs)) = accepted {
                rect = next;
                rows = rs;
                cols = cs;
            } else {
                break;
            }
        }
    }
    (rect, rows, cols)
}

fn refine_axis(profile: &[f64], start: u32, length: u32, cells: usize) -> (u32, u32) {
    if cells < 2 {
        return (start, length);
    }
    let estimated = length as f64 / cells as f64;
    let periods = [
        estimated,
        (estimated * 4.0).round() / 4.0,
        (estimated * 2.0).round() / 2.0,
        estimated.round(),
    ];
    let mut best = (start, length);
    let mut best_score = f64::NEG_INFINITY;
    for period in periods {
        for offset in -(estimated * 0.4) as i32..=(estimated * 0.4) as i32 {
            let origin = start as i32 + offset;
            if origin < 0 {
                continue;
            }
            let extent = (period * cells as f64).round() as u32;
            if origin as usize + extent as usize > profile.len() {
                continue;
            }
            let score = (1..cells)
                .map(|i| profile[(origin as f64 + period * i as f64).round() as usize])
                .sum::<f64>()
                / (cells - 1) as f64;
            if score > best_score {
                best_score = score;
                best = (origin as u32, extent);
            }
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_header(bytes: &[u8], header: u32, sides: u32) -> RgbImage {
        let source = image::load_from_memory(bytes).unwrap().to_rgb8();
        let board = image::imageops::crop_imm(&source, 20, 20, 288, 288).to_image();
        let mut screenshot =
            RgbImage::from_pixel(288 + sides * 2, header + 308, image::Rgb([60, 68, 76]));
        image::imageops::replace(&mut screenshot, &board, sides as i64, header as i64);
        // A toolbar edge one tile above the board is a plausible extra lattice line.
        for y in header - 32..header - 29 {
            for x in 0..screenshot.width() {
                screenshot.put_pixel(x, y, image::Rgb([180, 188, 195]));
            }
        }
        screenshot
    }

    #[test]
    fn toolbar_line_is_not_an_extra_board_row() {
        for bytes in [
            include_bytes!("../../tests/fixtures/classic.png").as_slice(),
            include_bytes!("../../tests/fixtures/modern-dark.png").as_slice(),
            include_bytes!("../../tests/fixtures/modern-light.png").as_slice(),
        ] {
            let screenshot = with_header(bytes, 64, 20);
            let (rect, rows, cols) = detect(&screenshot).unwrap();
            assert_eq!((rows, cols), (9, 9), "{rect:?}");
            // Beveled dividers can span several pixels; the cell labels below
            // verify the alignment rather than requiring one gradient pixel.
            assert!(rect.y.abs_diff(64) <= 4, "{rect:?}");
            let mut png = Cursor::new(Vec::new());
            image::DynamicImage::ImageRgb8(screenshot)
                .write_to(&mut png, image::ImageFormat::Png)
                .unwrap();
            let result = scan(ScanRequest {
                data: STANDARD.encode(png.into_inner()),
                crop: None,
                rows: None,
                cols: None,
                previous: None,
            })
            .unwrap();
            let truth: serde_json::Value =
                serde_json::from_str(include_str!("../../tests/fixtures/expected.json")).unwrap();
            let expected: Vec<i8> = serde_json::from_value(truth["cells"].clone()).unwrap();
            assert_eq!(result.cells, expected, "{rect:?}");
        }
    }

    #[test]
    fn frame_strips_and_tight_crops_at_different_scales() {
        let fixture = include_bytes!("../../tests/fixtures/modern-dark.png");
        for (header, sides, scale) in [
            (32, 0, 0.75),
            (32, 13, 1.25),
            (48, 32, 1.0),
            (64, 13, 0.75),
            (96, 0, 1.25),
            (96, 32, 1.0),
        ] {
            let screenshot = with_header(fixture, header, sides);
            let scaled = image::imageops::resize(
                &screenshot,
                (screenshot.width() as f64 * scale).round() as u32,
                (screenshot.height() as f64 * scale).round() as u32,
                FilterType::Triangle,
            );
            let (rect, rows, cols) = detect(&scaled).unwrap();
            assert_eq!(
                (rows, cols),
                (9, 9),
                "header={header} sides={sides} scale={scale}: {rect:?}"
            );
            assert!(
                rect.y.abs_diff((header as f64 * scale).round() as u32) <= 2,
                "{rect:?}"
            );
            let rotated = image::imageops::rotate90(&screenshot);
            let (rect, rows, cols) = detect(&rotated).unwrap();
            assert_eq!(
                (rows, cols),
                (9, 9),
                "rotated header={header} sides={sides}: {rect:?}"
            );
        }
    }

    #[test]
    fn real_blank_and_unreadable_edge_rows_are_retained() {
        let fixture = include_bytes!("../../tests/fixtures/modern-dark.png");
        let original = with_header(fixture, 64, 20);
        for blank in [false, true] {
            let mut screenshot = original.clone();
            for col in 0..9 {
                // Preserve real tile borders, replace just the symbol area.
                for y in 64 + 5..64 + 27 {
                    for x in 20 + col * 32 + 5..20 + col * 32 + 27 {
                        let color = if blank {
                            [23, 31, 41]
                        } else if (x + y) % 4 < 2 {
                            [255, 255, 255]
                        } else {
                            [0, 0, 0]
                        };
                        screenshot.put_pixel(x, y, image::Rgb(color));
                    }
                }
            }
            let (rect, rows, cols) = detect(&screenshot).unwrap();
            assert_eq!((rows, cols), (9, 9), "blank={blank}: {rect:?}");
            assert!(rect.y.abs_diff(64) <= 1, "{rect:?}");
        }
    }

    #[test]
    fn fractional_scale_large_real_board_keeps_all_rows_and_columns() {
        let source = image::load_from_memory(include_bytes!(
            "../../tests/fixtures/dark-classic-progress.png"
        ))
        .unwrap()
        .to_rgb8();
        for scale in [0.75, 1.25] {
            let resized = image::imageops::resize(
                &source,
                (source.width() as f64 * scale).round() as u32,
                (source.height() as f64 * scale).round() as u32,
                FilterType::Triangle,
            );
            let (rect, rows, cols) = detect(&resized).unwrap();
            assert_eq!((rows, cols), (33, 48), "scale={scale}: {rect:?}");
        }
    }

    #[test]
    fn blank_revealed_strip_without_dividers_is_not_trimmed() {
        let mut screenshot = with_header(
            include_bytes!("../../tests/fixtures/modern-dark.png"),
            64,
            20,
        );
        for y in 64..96 {
            for x in 20..308 {
                screenshot.put_pixel(x, y, image::Rgb([23, 31, 41]));
            }
        }
        let (rect, rows, cols) = trim_frame(
            &screenshot,
            (
                Rect {
                    x: 20,
                    y: 64,
                    width: 288,
                    height: 288,
                },
                9,
                9,
            ),
        );
        assert_eq!((rows, cols), (9, 9));
        assert_eq!(rect.y, 64);
    }

    #[test]
    fn custom_fully_covered_boards_and_maximum_size_ignore_toolbar() {
        let source = image::load_from_memory(include_bytes!("../../tests/fixtures/classic.png"))
            .unwrap()
            .to_rgb8();
        let covered = image::imageops::crop_imm(&source, 148, 20, 32, 32).to_image();
        let covered = image::imageops::resize(&covered, 16, 16, FilterType::Triangle);
        for (rows, cols) in [(16, 30), (50, 50)] {
            let mut screenshot =
                RgbImage::from_pixel(cols * 16 + 40, rows * 16 + 96, image::Rgb([60, 68, 76]));
            for y in 0..rows {
                for x in 0..cols {
                    image::imageops::replace(
                        &mut screenshot,
                        &covered,
                        (20 + x * 16) as i64,
                        (64 + y * 16) as i64,
                    );
                }
            }
            for y in 48..51 {
                for x in 0..screenshot.width() {
                    screenshot.put_pixel(x, y, image::Rgb([180, 188, 195]));
                }
            }
            let (rect, found_rows, found_cols) = detect(&screenshot).unwrap();
            assert_eq!(
                (found_rows, found_cols),
                (rows as usize, cols as usize),
                "{rect:?}"
            );
            assert!(rect.y.abs_diff(64) <= 1, "{rect:?}");
        }
    }
}
