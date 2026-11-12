use super::*;
use crate::solver::Board;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewReference {
    pub board: Board,
    pub rect: Rect,
    #[serde(default)]
    pub signatures: Option<Vec<String>>,
    #[serde(default)]
    pub data: Option<String>,
}
// Normalized tile appearance plus mean RGB. Small enough to keep with a session.
pub fn signatures(image: &RgbImage, rect: Rect, rows: usize, cols: usize) -> Vec<String> {
    (0..rows * cols)
        .map(|i| {
            let cell = cell_rect(rect, rows, cols, i);
            let inset = (cell.width.min(cell.height) / 10).max(1);
            let crop = image::imageops::crop_imm(
                image,
                cell.x + inset,
                cell.y + inset,
                cell.width.saturating_sub(inset * 2).max(1),
                cell.height.saturating_sub(inset * 2).max(1),
            )
            .to_image();
            let small = image::imageops::resize(&crop, 12, 16, FilterType::Triangle);
            let mut bytes = Vec::with_capacity(195);
            let mut sums = [0u32; 3];
            for p in small.pixels() {
                bytes.push(luminance(&p.0).round() as u8);
                for c in 0..3 {
                    sums[c] += p[c] as u32;
                }
            }
            bytes.extend(sums.map(|sum| (sum / 192) as u8));
            STANDARD.encode(bytes)
        })
        .collect()
}
fn similar(a: &str, b: &str) -> bool {
    if a.len() > 264 || b.len() > 264 {
        return false;
    }
    let (Ok(a), Ok(b)) = (STANDARD.decode(a), STANDARD.decode(b)) else {
        return false;
    };
    if a.len() != 195 || b.len() != 195 {
        return false;
    }
    let differences: Vec<_> = a[..192]
        .iter()
        .zip(&b[..192])
        .map(|(a, b)| a.abs_diff(*b) as usize)
        .collect();
    differences.iter().sum::<usize>() <= 144
        && differences.iter().filter(|&&d| d > 12).count() <= 1
        && a[192..]
            .iter()
            .zip(&b[192..])
            .all(|(a, b)| a.abs_diff(*b) <= 3)
}
pub(super) fn reuse(
    previous: Option<&ReviewReference>,
    rows: usize,
    cols: usize,
    current_signatures: &[String],
    tiles: &mut [Tile],
) -> usize {
    let Some(reference) = previous else {
        return 0;
    };
    if crate::solver::validate(&reference.board).is_err()
        || reference.board.rows != rows
        || reference.board.cols != cols
    {
        return 0;
    }
    let previous_signatures = if let Some(s) = &reference.signatures {
        s.clone()
    } else if let Some(data) = &reference.data {
        let Ok(image) = decode_image(data) else {
            return 0;
        };
        let rect = reference.rect;
        if rect.width / (cols as u32) < 6
            || rect.height / (rows as u32) < 6
            || rect
                .x
                .checked_add(rect.width)
                .is_none_or(|x| x > image.width())
            || rect
                .y
                .checked_add(rect.height)
                .is_none_or(|y| y > image.height())
        {
            return 0;
        }
        signatures(&image, rect, rows, cols)
    } else {
        return 0;
    };
    if previous_signatures.len() != tiles.len() || current_signatures.len() != tiles.len() {
        return 0;
    }
    let matches: Vec<_> = previous_signatures
        .iter()
        .zip(current_signatures)
        .map(|(a, b)| similar(a, b))
        .collect();
    let known = |v: i8| v >= 0 || v == -2 || v == -3;
    let anchors = (0..tiles.len())
        .filter(|&i| {
            known(reference.board.cells[i])
                && matches[i]
                && tiles[i].state == reference.board.cells[i]
                && tiles[i].score >= 0.8
        })
        .count();
    let known_count = reference.board.cells.iter().filter(|&&v| known(v)).count();
    if anchors < 3 && !(known_count > 0 && matches.iter().all(|v| *v)) {
        return 0;
    }
    // A clearly changed old clue/flag invalidates board-continuation reuse.
    if (0..tiles.len()).any(|i| {
        known(reference.board.cells[i])
            && !matches[i]
            && tiles[i].score >= 0.8
            && tiles[i].state != reference.board.cells[i]
    }) {
        return 0;
    }
    let mut count = 0;
    for (i, tile) in tiles.iter_mut().enumerate() {
        if known(reference.board.cells[i]) && matches[i] {
            tile.state = if reference.board.cells[i] == -3 {
                -2
            } else {
                reference.board.cells[i]
            };
            tile.score = 1.0;
            tile.note =
                "Previously accepted cell; unchanged appearance on the continuing board.".into();
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sig(value: u8) -> String {
        STANDARD.encode(vec![value; 195])
    }
    fn tile(v: i8, score: f64) -> Tile {
        Tile {
            state: v,
            score,
            note: String::new(),
            bg: [50; 3],
            empty: false,
            bevel: false,
        }
    }
    #[test]
    fn keeps_confirmed_unreadable_and_ambiguous_cells_but_not_new_ones() {
        let reference = ReviewReference {
            board: Board {
                rows: 1,
                cols: 6,
                cells: vec![1, 2, 3, 6, 7, -1],
                total_mines: None,
                trust_flags: true,
                revision: 0,
            },
            rect: Rect {
                x: 0,
                y: 0,
                width: 180,
                height: 30,
            },
            signatures: Some(vec![sig(50); 6]),
            data: None,
        };
        let mut current = vec![
            tile(1, 0.95),
            tile(2, 0.95),
            tile(3, 0.95),
            tile(-5, 0.3),
            tile(7, 0.65),
            tile(-5, 0.3),
        ];
        assert_eq!(
            reuse(Some(&reference), 1, 6, &vec![sig(50); 6], &mut current),
            5
        );
        assert_eq!(current[3].state, 6);
        assert_eq!(current[4].score, 1.0);
        assert_eq!(current[5].state, -5);
        let mut changed = vec![sig(50); 6];
        changed[3] = sig(110);
        current[3] = tile(-5, 0.3);
        reuse(Some(&reference), 1, 6, &changed, &mut current);
        assert_eq!(current[3].state, -5);
    }
    #[test]
    fn changed_board_and_malformed_fingerprints_do_not_inherit_review() {
        assert!(!similar(&sig(30), &sig(90)));
        assert!(!similar("invalid", "invalid"));
        let reference = ReviewReference {
            board: Board {
                rows: 1,
                cols: 4,
                cells: vec![1, 2, 3, 6],
                total_mines: None,
                trust_flags: true,
                revision: 0,
            },
            rect: Rect {
                x: 0,
                y: 0,
                width: 120,
                height: 30,
            },
            signatures: Some(vec![sig(50); 4]),
            data: None,
        };
        let mut current = vec![tile(2, 0.95), tile(2, 0.95), tile(3, 0.95), tile(-5, 0.3)];
        let mut signs = vec![sig(50); 4];
        signs[0] = sig(90);
        assert_eq!(reuse(Some(&reference), 1, 4, &signs, &mut current), 0);
        assert_eq!(current[3].state, -5);
    }
}
