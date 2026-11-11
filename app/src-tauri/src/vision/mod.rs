use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ImageReader, RgbImage, imageops::FilterType};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, sync::OnceLock};
mod counter;
pub use counter::CounterReading;

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    pub data: String,
    pub crop: Option<Rect>,
    pub rows: Option<usize>,
    pub cols: Option<usize>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<i8>,
    pub confidence: Vec<f64>,
    pub notes: Vec<String>,
    pub rect: Rect,
    pub message: String,
    pub counter: Option<CounterReading>,
    pub clue_colors: Vec<Option<String>>,
    pub auto_accept: bool,
    pub review_cells: Vec<usize>,
    pub review_reasons: Vec<String>,
}
#[derive(Deserialize)]
struct Template {
    label: i8,
    pixels: Vec<f64>,
}
static TEMPLATES: OnceLock<Vec<Template>> = OnceLock::new();

fn holes(pixels: &[f64], width: usize, height: usize) -> usize {
    let mut seen = vec![false; pixels.len()];
    let mut count = 0;
    for root in 0..pixels.len() {
        if seen[root] || pixels[root] >= 0.5 {
            continue;
        }
        let mut queue = vec![root];
        seen[root] = true;
        let mut touches = false;
        let mut area = 0;
        while let Some(i) = queue.pop() {
            area += 1;
            let x = i % width;
            let y = i / width;
            touches |= x == 0 || y == 0 || x + 1 == width || y + 1 == height;
            for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                    continue;
                }
                let next = ny as usize * width + nx as usize;
                if !seen[next] && pixels[next] < 0.5 {
                    seen[next] = true;
                    queue.push(next);
                }
            }
        }
        if !touches && area >= 4 {
            count += 1;
        }
    }
    count
}

fn open_four(pixels: &[f64]) -> bool {
    let region = |x0: usize, x1: usize, y0: usize, y1: usize| -> f64 {
        let mut ink = 0.0;
        for y in y0..y1 {
            for x in x0..x1 {
                ink += pixels[y * 16 + x];
            }
        }
        ink / ((x1 - x0) * (y1 - y0)) as f64
    };
    let bar = (8..16)
        .map(|y| {
            pixels[y * 16..y * 16 + 16]
                .iter()
                .filter(|&&p| p >= 0.5)
                .count()
        })
        .max()
        .unwrap_or(0);
    bar >= 14
        && region(0, 6, 17, 24) < 0.08
        && region(0, 7, 4, 9) > 0.18
        && region(10, 16, 4, 9) > 0.35
        && region(10, 16, 17, 24) > 0.5
}

fn luminance(p: &[u8; 3]) -> f64 {
    0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64
}
fn distance(a: &[u8; 3], b: &[u8; 3]) -> f64 {
    ((a[0] as f64 - b[0] as f64).powi(2)
        + (a[1] as f64 - b[1] as f64).powi(2)
        + (a[2] as f64 - b[2] as f64).powi(2))
    .sqrt()
}

fn lattice(profile: &[f64]) -> Option<(usize, usize, usize)> {
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
            let step = (next - start) as f64;
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

fn detect(image: &RgbImage) -> Result<(Rect, usize, usize), String> {
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
        let (Some((x, right, cols)), Some((y, bottom, rows))) = (lattice(&xp[k]), lattice(&yp[k]))
        else {
            continue;
        };
        if rows * cols > 2500 || (right - x) / cols < 6 || (bottom - y) / rows < 6 {
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
    // Strong glyph baselines can mimic a lattice on a nearly completed board.
    // Compare independent contrast passes by inspecting complete tile crops.
    let quality = |&(rect, rows, cols): &(Rect, usize, usize)| -> f64 {
        let stride = (rows * cols / 240).max(1);
        let mut score = 0.0;
        let mut count = 0;
        for i in (0..rows * cols).step_by(stride) {
            let tile = classify(image, cell_rect(rect, rows, cols, i));
            score += tile.score - if tile.state == -5 { 0.8 } else { 0.0 };
            count += 1;
        }
        let boundary_strength = |profile: &[f64], start: u32, length: u32, cells: usize| {
            let max = profile.iter().copied().fold(0.0, f64::max).max(1.0);
            (1..cells)
                .map(|i| {
                    profile
                        .get(start as usize + i * length as usize / cells)
                        .copied()
                        .unwrap_or(0.0)
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
    candidates.into_iter().max_by(|a,b|quality(a).total_cmp(&quality(b))).ok_or("I couldn't verify a reliable board grid. Crop the board and use Adjust grid if needed.".into())
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

fn cell_rect(rect: Rect, rows: usize, cols: usize, index: usize) -> Rect {
    let r = index / cols;
    let c = index % cols;
    let x = rect.x + c as u32 * rect.width / cols as u32;
    let y = rect.y + r as u32 * rect.height / rows as u32;
    Rect {
        x,
        y,
        width: rect.x + (c + 1) as u32 * rect.width / cols as u32 - x,
        height: rect.y + (r + 1) as u32 * rect.height / rows as u32 - y,
    }
}

fn dominant_background<'a>(tiles: impl Iterator<Item = &'a Tile>) -> Option<[u8; 3]> {
    let mut counts = std::collections::HashMap::new();
    for tile in tiles {
        *counts.entry(tile.bg).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(color, _)| color)
}

fn glyph_color(image: &RgbImage, rect: Rect, bg: [u8; 3]) -> Option<[u8; 3]> {
    let inset = (rect.width.min(rect.height) as f64 * 0.16).round().max(1.0) as u32;
    let mut counts = std::collections::HashMap::new();
    for y in rect.y + inset..rect.y + rect.height - inset {
        for x in rect.x + inset..rect.x + rect.width - inset {
            let color = image.get_pixel(x, y).0;
            if distance(&color, &bg) > 65.0 {
                *counts.entry(color).or_insert(0usize) += 1;
            }
        }
    }
    counts
        .into_iter()
        .max_by_key(|(_, n)| *n)
        .map(|(color, _)| color)
}

struct Tile {
    state: i8,
    score: f64,
    note: String,
    bg: [u8; 3],
    empty: bool,
    bevel: bool,
}
fn classify(img: &RgbImage, rect: Rect) -> Tile {
    let inset = (rect.width.min(rect.height) as f64 * 0.16).round().max(1.0) as u32;
    let x0 = (rect.x + inset).min(img.width() - 1);
    let y0 = (rect.y + inset).min(img.height() - 1);
    let x1 = (rect.x + rect.width - inset).min(img.width());
    let y1 = (rect.y + rect.height - inset).min(img.height());
    let mut channels = [Vec::new(), Vec::new(), Vec::new()];
    for y in y0..y1 {
        for x in x0..x1 {
            for c in 0..3 {
                channels[c].push(img.get_pixel(x, y)[c]);
            }
        }
    }
    let mut bg = [0u8; 3];
    for c in 0..3 {
        channels[c].sort_unstable();
        bg[c] = channels[c]
            .get(channels[c].len() / 2)
            .copied()
            .unwrap_or(128);
    }
    // Thick colored clues can occupy over half the inner crop. Prefer the
    // dominant neutral background when it is present, rather than its glyph.
    let mut neutral = std::collections::HashMap::<[u8; 3], usize>::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let p = img.get_pixel(x, y).0;
            if p.iter().max().unwrap() - p.iter().min().unwrap() < 28 {
                let key = p.map(|v| v / 8 * 8);
                *neutral.entry(key).or_default() += 1;
            }
        }
    }
    if let Some((color, count)) = neutral.into_iter().max_by_key(|(_, count)| *count) {
        if count >= ((x1 - x0) * (y1 - y0) / 8) as usize {
            bg = color;
        }
    }
    let sample = |x: u32, y: u32| {
        luminance(
            &img.get_pixel(x.min(img.width() - 1), y.min(img.height() - 1))
                .0,
        )
    };
    let bevel = sample(rect.x + 1, rect.y + 1)
        - sample(
            rect.x + rect.width.saturating_sub(2),
            rect.y + rect.height.saturating_sub(2),
        )
        > 28.0;
    let mut points = Vec::new();
    let mut red = 0usize;
    let mut blue = 0usize;
    let mut green = 0usize;
    let mut red_points = Vec::new();
    let mut pole_points = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            let p = &img.get_pixel(x, y).0;
            if distance(p, &bg) > 65.0 {
                points.push((x, y));
                let [r, g, b] = p.map(|v| v as f64);
                if r > g * 1.35 + 20.0 && r > b * 1.3 + 20.0 {
                    red += 1;
                    red_points.push((x, y));
                }
                if b > r * 1.3 + 20.0 && b > g * 1.15 + 10.0 {
                    blue += 1;
                }
                if g > r * 1.2 + 12.0 && g > b * 1.1 + 5.0 {
                    green += 1;
                }
                if r.max(g).max(b) - r.min(g).min(b) < 35.0
                    && (luminance(p) - luminance(&bg)).abs() > 60.0
                {
                    pole_points.push((x, y));
                }
            }
        }
    }
    if points.len() < 2 {
        return Tile {
            state: -1,
            score: if bevel { 0.87 } else { 0.45 },
            note: "Blank-looking tile; confirm covered versus revealed zero.".into(),
            bg,
            empty: true,
            bevel,
        };
    }
    // A flag has a compact colored pennant above a contrasting pole/base.
    // A colored digit such as 3 spans the glyph height and has no neutral base.
    if red_points.len() >= 3 {
        let top = red_points.iter().map(|p| p.1).min().unwrap();
        let bottom = red_points.iter().map(|p| p.1).max().unwrap();
        let lower: Vec<_> = pole_points.iter().filter(|p| p.1 > bottom).collect();
        let base_width = lower
            .iter()
            .map(|p| p.0)
            .max()
            .unwrap_or(0)
            .saturating_sub(lower.iter().map(|p| p.0).min().unwrap_or(0));
        if bottom < rect.y + rect.height * 2 / 3
            && bottom - top >= rect.height / 10
            && lower.len() >= rect.width as usize / 4
            && base_width >= rect.width / 3
        {
            return Tile {
                state: -2,
                score: 0.96,
                note:
                    "Flag recognized from its pennant, pole, and base. Flags are confirmed mines."
                        .into(),
                bg,
                empty: false,
                bevel,
            };
        }
    }
    let bx0 = points.iter().map(|p| p.0).min().unwrap();
    let bx1 = points.iter().map(|p| p.0).max().unwrap();
    let by0 = points.iter().map(|p| p.1).min().unwrap();
    let by1 = points.iter().map(|p| p.1).max().unwrap();
    let mut mask = image::GrayImage::new(bx1 - bx0 + 1, by1 - by0 + 1);
    for &(x, y) in &points {
        mask.put_pixel(x - bx0, y - by0, image::Luma([255]));
    }
    let normalized = image::imageops::resize(&mask, 16, 24, FilterType::Triangle);
    let vector: Vec<f64> = normalized.pixels().map(|p| p[0] as f64 / 255.0).collect();
    let observed_holes = holes(&vector, 16, 24);
    if observed_holes == 0 && open_four(&vector) {
        return Tile {
            state: 4,
            score: 0.94,
            note: "Open-top 4 recognized from its crossbar and lower right stem.".into(),
            bg,
            empty: false,
            bevel,
        };
    }
    let references =
        TEMPLATES.get_or_init(|| serde_json::from_str(include_str!("digits.json")).unwrap());
    let mut scores = [f64::INFINITY; 9];
    for template in references {
        let mut error = vector
            .iter()
            .zip(&template.pixels)
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            / 384.0;
        error += observed_holes.abs_diff(holes(&template.pixels, 16, 24)) as f64 * 0.10;
        // Color is supporting evidence. It never selects a clue without a shape match.
        let fraction = points.len() as f64;
        let favored = if blue as f64 / fraction > 0.65 {
            Some(if luminance(&bg) > 100.0 {
                if blue > 0
                    && points
                        .iter()
                        .map(|&(x, y)| img.get_pixel(x, y)[2] as usize)
                        .sum::<usize>()
                        / points.len()
                        > 160
                {
                    1
                } else {
                    4
                }
            } else {
                1
            })
        } else if green as f64 / fraction > 0.65 {
            Some(2)
        } else if red as f64 / fraction > 0.65 {
            Some(3)
        } else {
            None
        };
        if let Some(label) = favored {
            if template.label == label {
                error *= 0.78;
            }
        }
        scores[template.label as usize] = scores[template.label as usize].min(error);
    }
    let mut ranked: Vec<(usize, f64)> = (1..=8).map(|v| (v, scores[v])).collect();
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1));
    let (label, error) = ranked[0];
    let margin = ranked[1].1 - error;
    // Reject unfamiliar glyphs instead of quietly inventing a clue.
    if error > 0.23 || (bevel && (red > 0 || error > 0.09)) {
        return Tile {
            state: -5,
            score: 0.3,
            note: "Unrecognized symbol or flag. Select its state in the editor.".into(),
            bg,
            empty: false,
            bevel,
        };
    }
    let score = (0.95 - error * 1.7 + (margin * 1.5).min(0.08)).clamp(0.35, 0.97);
    Tile {
        state: label as i8,
        score,
        note: format!("Shape match for {label}; verify against the screenshot."),
        bg,
        empty: false,
        bevel,
    }
}

pub fn scan(request: ScanRequest) -> Result<ScanResult, String> {
    if request.data.len() > 28_000_000 {
        return Err("Choose an image smaller than 20 MB.".into());
    }
    let data = request.data.split(',').next_back().unwrap_or(&request.data);
    let bytes = STANDARD
        .decode(data)
        .map_err(|_| "Could not read image data.")?;
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut reader = reader;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(6000);
    limits.max_image_height = Some(6000);
    limits.max_alloc = Some(160_000_000);
    reader.limits(limits);
    let rgba=reader.decode().map_err(|_|"This image could not be decoded or is too large. Use a PNG, JPEG, or WebP screenshot up to 6000 × 6000.")?.to_rgba8();
    let mut img = RgbImage::new(rgba.width(), rgba.height());
    for (x, y, p) in rgba.enumerate_pixels() {
        let alpha = p[3] as f64 / 255.0;
        img.put_pixel(
            x,
            y,
            image::Rgb([0, 1, 2].map(|c| (p[c] as f64 * alpha + 255.0 * (1.0 - alpha)) as u8)),
        );
    }
    let full = Rect {
        x: 0,
        y: 0,
        width: img.width(),
        height: img.height(),
    };
    let crop = request.crop.unwrap_or(full);
    if crop.width < 6
        || crop.height < 6
        || crop
            .x
            .checked_add(crop.width)
            .is_none_or(|v| v > img.width())
        || crop
            .y
            .checked_add(crop.height)
            .is_none_or(|v| v > img.height())
    {
        return Err("Select a crop inside the screenshot, at least 6 pixels wide and high.".into());
    }
    let cropped =
        image::imageops::crop_imm(&img, crop.x, crop.y, crop.width, crop.height).to_image();
    let (rect, rows, cols) = match (request.rows, request.cols) {
        (Some(rows), Some(cols)) => {
            if rows == 0 || cols == 0 || rows > 100 || cols > 100 || rows * cols > 2500 {
                return Err("Use 1–100 rows and columns, up to 2,500 cells.".into());
            }
            (
                Rect {
                    x: 0,
                    y: 0,
                    width: crop.width,
                    height: crop.height,
                },
                rows,
                cols,
            )
        }
        (None, None) => detect(&cropped)?,
        _ => {
            return Err(
                "Set both rows and columns, or leave both blank for automatic detection.".into(),
            );
        }
    };
    if rect.width / (cols as u32) < 6 || rect.height / (rows as u32) < 6 {
        return Err(
            "The cells are too small to recognize reliably. Try a higher-resolution screenshot."
                .into(),
        );
    }
    let mut tiles = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let x = rect.x + (c as u32 * rect.width / cols as u32);
            let y = rect.y + (r as u32 * rect.height / rows as u32);
            let right = rect.x + ((c + 1) as u32 * rect.width / cols as u32);
            let bottom = rect.y + ((r + 1) as u32 * rect.height / rows as u32);
            tiles.push(classify(
                &cropped,
                Rect {
                    x,
                    y,
                    width: right - x,
                    height: bottom - y,
                },
            ));
        }
    }
    let open_bg = dominant_background(
        tiles
            .iter()
            .filter(|t| t.state >= 1 && !t.bevel && t.score > 0.80),
    );
    let covered_bg = dominant_background(
        tiles
            .iter()
            .filter(|t| (t.empty && t.bevel) || t.state == -2),
    );
    let distinct_backgrounds =
        matches!((open_bg,covered_bg),(Some(a),Some(b)) if distance(&a,&b)>20.0);
    for tile in &mut tiles {
        let revealed = distinct_backgrounds
            && open_bg.is_some_and(|bg| {
                distance(&tile.bg, &bg) + 8.0 < distance(&tile.bg, &covered_bg.unwrap())
            });
        let covered = distinct_backgrounds
            && covered_bg.is_some_and(|bg| {
                distance(&tile.bg, &bg) + 8.0 < distance(&tile.bg, &open_bg.unwrap())
            });
        if !tile.empty && ((covered && tile.state >= 0) || (revealed && tile.state == -2)) {
            tile.state = -5;
            tile.score = 0.25;
            tile.note = "Symbol and cell background disagree. This cell needs review.".into();
        } else if tile.empty {
            if revealed || (!tile.bevel && open_bg.is_some_and(|bg| distance(&bg, &tile.bg) < 20.0))
            {
                tile.state = 0;
                tile.score = 0.98;
                tile.note =
                    "Revealed empty cell confirmed by its background. No individual review needed."
                        .into();
            } else if covered || tile.bevel {
                tile.state = -1;
                tile.score = 0.98;
                tile.note = "Covered cell confirmed by background and border.".into();
            } else if open_bg.is_some() {
                tile.state = -1;
                tile.score = 0.85;
                tile.note = "Covered cell with a different background from revealed cells.".into();
            }
        }
    }
    let mut color_votes: Vec<std::collections::HashMap<[u8; 3], usize>> =
        (0..9).map(|_| std::collections::HashMap::new()).collect();
    for (i, tile) in tiles.iter().enumerate() {
        if tile.state > 0 && tile.score > 0.8 {
            if let Some(color) = glyph_color(&cropped, cell_rect(rect, rows, cols, i), tile.bg) {
                *color_votes[tile.state as usize].entry(color).or_default() += 1;
            }
        }
    }
    let clue_colors = color_votes
        .into_iter()
        .map(|votes| {
            votes
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .map(|(c, _)| format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2]))
        })
        .collect();
    let absolute_rect = Rect {
        x: crop.x + rect.x,
        y: crop.y + rect.y,
        ..rect
    };
    let flag_count = tiles.iter().filter(|t| t.state == -2).count();
    let counter = counter::read_counter(&img, absolute_rect, flag_count, rows * cols);
    let cells: Vec<_> = tiles.iter().map(|t| t.state).collect();
    let confidence: Vec<_> = tiles.iter().map(|t| t.score).collect();
    let (auto_accept, review_cells, review_reasons) =
        review_gate(rows, cols, &cells, &confidence, counter.as_ref());
    Ok(ScanResult {
        rows,
        cols,
        cells,
        confidence,
        notes: tiles.into_iter().map(|t| t.note).collect(),
        rect: absolute_rect,
        counter,
        clue_colors,
        auto_accept,
        review_cells,
        review_reasons,
        message: if auto_accept {
            "Scan accepted automatically: no unreadable or ambiguous cells detected."
        } else {
            "Scan complete. Review the highlighted symbols and scan checks."
        }
        .into(),
    })
}

fn review_gate(
    rows: usize,
    cols: usize,
    cells: &[i8],
    confidence: &[f64],
    counter: Option<&CounterReading>,
) -> (bool, Vec<usize>, Vec<String>) {
    let mut review: Vec<_> = cells
        .iter()
        .enumerate()
        .filter(|(i, v)| {
            **v == -5
                || confidence
                    .get(*i)
                    .is_none_or(|score| !score.is_finite() || *score < 0.8)
        })
        .map(|(i, _)| i)
        .collect();
    let mut reasons = Vec::new();
    if !review.is_empty() {
        reasons.push("Some cells are unreadable or their visual matches are ambiguous.".into());
    }
    for (i, &value) in cells.iter().enumerate().filter(|(_, v)| **v >= 0) {
        let around = crate::solver::neighbors(i, rows, cols);
        let flags = around
            .iter()
            .filter(|&&v| cells[v] == -2 || cells[v] == -3)
            .count();
        let unknown = around
            .iter()
            .filter(|&&v| cells[v] == -1 || cells[v] == -5)
            .count();
        if flags > value as usize || flags + unknown < value as usize {
            review.push(i);
        }
    }
    if review.iter().any(|&i| cells[i] >= 0) {
        reasons.push("Detected clues and flags need a consistency review.".into());
    }
    if let Some(counter) = counter {
        let flag_count = cells.iter().filter(|&&v| v == -2 || v == -3).count();
        let covered = cells.iter().filter(|&&v| v == -1 || v == -5).count();
        if counter
            .total_mines
            .is_none_or(|total| total < flag_count || total > flag_count + covered)
        {
            reasons.push("The detected remaining-flags counter does not fit this board.".into());
        }
    }
    review.sort_unstable();
    review.dedup();
    (review.is_empty() && reasons.is_empty(), review, reasons)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scan_acceptance_requires_confidence_and_consistency() {
        assert!(review_gate(1, 3, &[1, -2, -1], &[0.95; 3], None).0);
        assert!(!review_gate(1, 3, &[1, -2, -1], &[0.79, 0.95, 0.95], None).0);
        assert!(!review_gate(1, 3, &[1, -5, -1], &[0.99; 3], None).0);
        assert!(!review_gate(1, 3, &[0, -2, -1], &[0.99; 3], None).0);
    }
    #[test]
    fn detects_regular_spacing() {
        let mut p = vec![0.0; 320];
        for x in (19..=259).step_by(24) {
            p[x] = 30.0;
        }
        assert_eq!(lattice(&p), Some((19, 259, 10)));
    }
    #[test]
    fn rejects_plain_image() {
        assert!(detect(&RgbImage::from_pixel(100, 100, image::Rgb([30, 30, 30]))).is_err());
    }
    #[test]
    fn validates_decode() {
        assert!(
            scan(ScanRequest {
                data: "bad".into(),
                crop: None,
                rows: None,
                cols: None
            })
            .is_err()
        );
    }
    #[test]
    fn recognizes_empty_classic_bevel() {
        let mut image = RgbImage::from_pixel(24, 24, image::Rgb([192, 192, 192]));
        for i in 0..24 {
            for t in 0..2 {
                image.put_pixel(i, t, image::Rgb([255, 255, 255]));
                image.put_pixel(t, i, image::Rgb([255, 255, 255]));
                image.put_pixel(i, 23 - t, image::Rgb([100, 100, 100]));
                image.put_pixel(23 - t, i, image::Rgb([100, 100, 100]));
            }
        }
        let tile = classify(
            &image,
            Rect {
                x: 0,
                y: 0,
                width: 24,
                height: 24,
            },
        );
        assert_eq!(tile.state, -1);
        assert!(tile.bevel);
    }
    #[test]
    fn screenshot_fixtures_manual_grid() {
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/expected.json")).unwrap();
        for bytes in [
            include_bytes!("../../tests/fixtures/classic.png").as_slice(),
            include_bytes!("../../tests/fixtures/modern-light.png").as_slice(),
            include_bytes!("../../tests/fixtures/modern-dark.png").as_slice(),
        ] {
            let result = scan(ScanRequest {
                data: STANDARD.encode(bytes),
                crop: Some(Rect {
                    x: 20,
                    y: 20,
                    width: 288,
                    height: 288,
                }),
                rows: Some(9),
                cols: Some(9),
            })
            .unwrap();
            let truth: Vec<i8> = serde_json::from_value(expected["cells"].clone()).unwrap();
            assert_eq!(result.cells, truth);
        }
    }
    #[test]
    fn screenshot_fixtures_auto_geometry() {
        for bytes in [
            include_bytes!("../../tests/fixtures/classic.png").as_slice(),
            include_bytes!("../../tests/fixtures/modern-light.png").as_slice(),
            include_bytes!("../../tests/fixtures/modern-dark.png").as_slice(),
        ] {
            let result = scan(ScanRequest {
                data: STANDARD.encode(bytes),
                crop: None,
                rows: None,
                cols: None,
            })
            .unwrap();
            assert_eq!((result.rows, result.cols), (9, 9), "{:?}", result.rect);
        }
    }
    #[test]
    fn detects_tightly_cropped_board() {
        let image = image::load_from_memory(include_bytes!("../../tests/fixtures/classic.png"))
            .unwrap()
            .to_rgb8();
        let crop = image::imageops::crop_imm(&image, 20, 20, 288, 288).to_image();
        let (_, rows, cols) = detect(&crop).unwrap();
        assert_eq!((rows, cols), (9, 9));
    }
    #[test]
    fn user_dark_classic_flags_clues_and_remaining_counter() {
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/dark-classic-user.json"))
                .unwrap();
        let result = scan(ScanRequest {
            data: STANDARD.encode(include_bytes!("../../tests/fixtures/dark-classic-user.png")),
            crop: None,
            rows: None,
            cols: None,
        })
        .unwrap();
        assert_eq!((result.rows, result.cols), (33, 48));
        let truth: Vec<i8> = serde_json::from_value(expected["cells"].clone()).unwrap();
        let mismatches: Vec<_> = result
            .cells
            .iter()
            .zip(&truth)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| format!("R{}C{}: {a} expected {b}", i / 48 + 1, i % 48 + 1))
            .collect();
        assert!(mismatches.is_empty(), "{}", mismatches.join(", "));
        let counter = result.counter.expect("top-left seven-segment counter");
        assert_eq!(counter.value, 335);
        assert_eq!(counter.flag_count, 27);
        assert_eq!(counter.total_mines, Some(362));
    }
    #[test]
    fn progressed_board_keeps_grid_phase_background_states_and_palette() {
        let truth: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/dark-classic-progress.json"
        ))
        .unwrap();
        let result = scan(ScanRequest {
            data: STANDARD.encode(include_bytes!(
                "../../tests/fixtures/dark-classic-progress.png"
            )),
            crop: None,
            rows: None,
            cols: None,
        })
        .unwrap();
        assert_eq!((result.rows, result.cols), (33, 48));
        assert_eq!(result.rect.y, 101);
        let expected: Vec<i8> = serde_json::from_value(truth["cells"].clone()).unwrap();
        let differences: Vec<_> = result
            .cells
            .iter()
            .zip(&expected)
            .enumerate()
            .filter(|(_, (a, b))| a != b)
            .map(|(i, (a, b))| format!("R{}C{} {a} != {b}", i / 48 + 1, i % 48 + 1))
            .collect();
        assert!(differences.is_empty(), "{}", differences.join(", "));
        assert!(
            result
                .cells
                .iter()
                .zip(&result.confidence)
                .filter(|(v, _)| **v == 0 || **v == -1)
                .all(|(_, score)| *score >= 0.8)
        );
        assert_eq!(result.clue_colors[1].as_deref(), Some("#7cc7ff"));
        assert_eq!(result.clue_colors[2].as_deref(), Some("#66c266"));
        assert_eq!(result.clue_colors[3].as_deref(), Some("#ff7788"));
        assert_eq!(result.clue_colors[4].as_deref(), Some("#ee88ff"));
        let counter = result.counter.unwrap();
        assert_eq!(counter.value, 42);
        assert_eq!(counter.flag_count, 320);
        assert_eq!(counter.total_mines, Some(362));
    }
}
