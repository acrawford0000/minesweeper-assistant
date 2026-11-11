use crate::solver::{Board, COVERED, FLAG, MINE, validate};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub kind: String,
    pub preserved_known: usize,
    pub newly_revealed: usize,
    pub new_flags: usize,
    pub changed_known: usize,
    pub unchanged_covered: usize,
    pub message: String,
}
pub fn compare(previous: &Board, current: &Board) -> Result<Comparison, String> {
    validate(previous)?;
    validate(current)?;
    let mut out = Comparison {
        kind: "different".into(),
        preserved_known: 0,
        newly_revealed: 0,
        new_flags: 0,
        changed_known: 0,
        unchanged_covered: 0,
        message: String::new(),
    };
    if previous.rows != current.rows || previous.cols != current.cols {
        out.message = "Different board dimensions. Analyzing this position independently.".into();
        return Ok(out);
    }
    for (&before, &after) in previous.cells.iter().zip(&current.cells) {
        let before_flag = before == FLAG || before == MINE;
        let after_flag = after == FLAG || after == MINE;
        if before >= 0 || before_flag {
            if before == after || (before_flag && after_flag) {
                out.preserved_known += 1;
            } else {
                out.changed_known += 1;
            }
        }
        if before < 0 && after >= 0 {
            out.newly_revealed += 1;
        }
        if !before_flag && after_flag {
            out.new_flags += 1;
        }
        if before == COVERED && after == COVERED {
            out.unchanged_covered += 1;
        }
    }
    if out.changed_known > 0 {
        out.message = format!(
            "{} previous clues or flags differ. Treating this as a changed position; only exactly matching frontier constraints can be reused.",
            out.changed_known
        );
    } else if out.preserved_known < 3 {
        out.kind = "uncertain".into();
        out.message="Too few matching revealed clues or flags to identify a continuing game. Exact constraint matches are still safe to reuse.".into();
    } else if previous.cells == current.cells {
        out.kind = "same".into();
        out.message =
            "The reconstructed cells match the previous board. Mine totals are applied afresh."
                .into();
    } else {
        out.kind = "progress".into();
        out.message = format!(
            "Looks like the same board: {} newly revealed squares, {} new flags, and {} preserved clues or flags. Matching frontiers can be reused; the shared covered pool is reweighted with current information.",
            out.newly_revealed, out.new_flags, out.preserved_known
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn board(cells: Vec<i8>) -> Board {
        Board {
            rows: 2,
            cols: 4,
            cells,
            total_mines: Some(2),
            trust_flags: true,
            revision: 0,
        }
    }
    #[test]
    fn recognizes_progress_without_assuming_hidden_cells_are_unchanged_odds() {
        let old = board(vec![0, 1, FLAG, COVERED, 0, COVERED, COVERED, COVERED]);
        let mut new = old.clone();
        new.cells[5] = 1;
        let c = compare(&old, &new).unwrap();
        assert_eq!(c.kind, "progress");
        assert_eq!(c.newly_revealed, 1);
        assert_eq!(c.preserved_known, 4);
        assert_eq!(c.unchanged_covered, 3);
        new.cells[1] = 2;
        assert_eq!(compare(&old, &new).unwrap().kind, "different");
    }
    #[test]
    fn identical_cells_still_allow_changed_counter_and_sparse_boards_are_uncertain() {
        let old = board(vec![0, 1, FLAG, COVERED, 0, COVERED, COVERED, COVERED]);
        let mut new = old.clone();
        new.total_mines = Some(3);
        assert_eq!(compare(&old, &new).unwrap().kind, "same");
        assert_eq!(
            compare(&board(vec![COVERED; 8]), &board(vec![COVERED; 8]))
                .unwrap()
                .kind,
            "uncertain"
        );
        new.cols = 2;
        new.rows = 4;
        assert_eq!(compare(&old, &new).unwrap().kind, "different");
    }
}
