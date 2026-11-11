use super::Rect;
use image::RgbImage;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterReading {
    pub value: i32,
    pub text: String,
    pub rect: Rect,
    pub flag_count: usize,
    pub total_mines: Option<usize>,
}

fn runs(values: &[bool]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start = None;
    for (i, &active) in values.iter().chain(std::iter::once(&false)).enumerate() {
        if active && start.is_none() {
            start = Some(i);
        }
        if !active {
            if let Some(s) = start.take() {
                out.push((s, i));
            }
        }
    }
    out
}

pub fn read_counter(
    img: &RgbImage,
    board: Rect,
    flags: usize,
    cells: usize,
) -> Option<CounterReading> {
    // Search above the board, on the left. The timer on the right is excluded.
    let width = (board.x + board.width * 45 / 100).min(img.width()) as usize;
    let height = board.y.min(img.height()) as usize;
    if height < 10 || width < 10 {
        return None;
    }
    let red = |x: usize, y: usize| {
        let [r, g, b] = img.get_pixel(x as u32, y as u32).0.map(|v| v as f64);
        r > 100.0 && r > g * 1.8 + 40.0 && r > b * 1.8 + 40.0
    };
    let active_rows: Vec<_> = (0..height)
        .map(|y| (0..width).filter(|&x| red(x, y)).count() >= 3)
        .collect();
    let bands = runs(&active_rows);
    for (top, bottom) in bands.into_iter().filter(|(a, b)| b - a >= 12) {
        let columns: Vec<_> = (0..width)
            .map(|x| (top..bottom).any(|y| red(x, y)))
            .collect();
        let groups = runs(&columns);
        if groups.is_empty() || groups.len() > 5 {
            continue;
        }
        let mut text = String::new();
        let mut valid = true;
        for &(left, right) in &groups {
            let w = (right - left) as f64;
            let h = (bottom - top) as f64;
            if w / h < 0.27 {
                text.push('1');
                continue;
            }
            let regions = [
                (0.25, 0.75, 0.02, 0.17),
                (0.72, 1.0, 0.19, 0.41),
                (0.72, 1.0, 0.59, 0.81),
                (0.25, 0.75, 0.83, 0.98),
                (0.0, 0.28, 0.59, 0.81),
                (0.0, 0.28, 0.19, 0.41),
                (0.25, 0.75, 0.42, 0.58),
            ];
            let mut bits = 0u8;
            for (i, (x0, x1, y0, y1)) in regions.into_iter().enumerate() {
                let mut on = 0;
                let mut n = 0;
                for y in top + (y0 * h) as usize..top + (y1 * h) as usize {
                    for x in left + (x0 * w) as usize..left + (x1 * w) as usize {
                        n += 1;
                        if red(x, y) {
                            on += 1;
                        }
                    }
                }
                if n > 0 && on as f64 / n as f64 > 0.28 {
                    bits |= 1 << i;
                }
            }
            let patterns = [
                0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101,
                0b0000111, 0b1111111, 0b1101111,
            ];
            if bits == 0b1000000 && text.is_empty() {
                text.push('-');
            } else if let Some(digit) = patterns.iter().position(|&p| p == bits) {
                text.push(char::from(b'0' + digit as u8));
            } else {
                valid = false;
                break;
            }
        }
        if !valid {
            continue;
        }
        let Ok(value) = text.parse::<i32>() else {
            continue;
        };
        let total = value
            .checked_add(flags as i32)
            .filter(|&n| n >= 0 && n as usize <= cells)
            .map(|n| n as usize);
        let left = groups.first()?.0;
        let right = groups.last()?.1;
        return Some(CounterReading {
            value,
            text,
            rect: Rect {
                x: left as u32,
                y: top as u32,
                width: (right - left) as u32,
                height: (bottom - top) as u32,
            },
            flag_count: flags,
            total_mines: total,
        });
    }
    None
}
