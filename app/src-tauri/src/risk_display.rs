use crate::solver::{Board, COVERED, Odds};
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RiskDisplay {
    pub rounded: u8,
    pub color: String,
    pub highlight: String,
    pub muted: bool,
}

fn risk_color(probability: f64) -> String {
    let green = [54.0, 249.0, 163.0];
    let orange = [255.0, 154.0, 36.0];
    let red = [255.0, 48.0, 56.0];
    let (from, to, t) = if probability <= 0.5 {
        (green, orange, probability * 2.0)
    } else {
        (orange, red, (probability - 0.5) * 2.0)
    };
    let rgb: [u8; 3] = std::array::from_fn(|i| (from[i] + (to[i] - from[i]) * t).round() as u8);
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

/// Presentation decisions use unrounded odds; rounding never changes certainty.
pub fn apply(board: &Board, odds: &mut [Odds]) {
    let candidates: Vec<_> = odds
        .iter()
        .enumerate()
        .filter(|(i, o)| board.cells[*i] == COVERED && o.probability.is_some())
        .map(|(_, o)| o.probability.unwrap())
        .collect();
    let min = candidates.iter().copied().reduce(f64::min);
    let max = candidates.iter().copied().reduce(f64::max);
    let spread = matches!((min,max),(Some(a),Some(b)) if b-a>1e-12);
    let mut bands = [0usize; 51];
    for &p in &candidates {
        bands[((p * 100.0).round() as usize + 1) / 2] += 1;
    }
    let common_threshold = (candidates.len() / 12).max(6);
    for (i, odd) in odds.iter_mut().enumerate() {
        let Some(p) = odd.probability else {
            continue;
        };
        let rounded = (p.clamp(0.0, 1.0) * 100.0).round() as u8;
        let eligible = board.cells[i] == COVERED;
        let highlight = if odd.kind == "safe" {
            "safe"
        } else if odd.kind == "mine" {
            "mine"
        } else if eligible && spread && min.is_some_and(|v| (v - p).abs() < 1e-12) {
            "best"
        } else if eligible && spread && max.is_some_and(|v| (v - p).abs() < 1e-12) {
            "worst"
        } else {
            "none"
        };
        let muted = eligible
            && highlight == "none"
            && bands[(rounded as usize + 1) / 2] >= common_threshold;
        odd.display = Some(RiskDisplay {
            rounded,
            color: if muted {
                "#8b96a7".into()
            } else {
                risk_color(p)
            },
            highlight: highlight.into(),
            muted,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn board(n: usize) -> Board {
        Board {
            rows: 1,
            cols: n,
            cells: vec![COVERED; n],
            total_mines: None,
            trust_flags: true,
            revision: 0,
        }
    }
    fn odds(values: &[f64]) -> Vec<Odds> {
        values
            .iter()
            .map(|&p| Odds {
                probability: Some(p),
                kind: "risk".into(),
                reason: String::new(),
                numerator: None,
                display: None,
            })
            .collect()
    }
    #[test]
    fn highlights_only_extremes_and_mutes_common_middle_band() {
        let mut o = odds(&[0.01, 0.01, 0.249, 0.251, 0.25, 0.25, 0.25, 0.25, 0.88]);
        apply(&board(o.len()), &mut o);
        assert_eq!(o[0].display.as_ref().unwrap().highlight, "best");
        assert_eq!(o[1].display.as_ref().unwrap().highlight, "best");
        assert_eq!(o[8].display.as_ref().unwrap().highlight, "worst");
        assert!(o[2..8].iter().all(|o| o.display.as_ref().unwrap().muted));
    }
    #[test]
    fn rounding_does_not_promote_guesses_to_certainties() {
        let mut o = odds(&[0.004, 0.996]);
        apply(&board(2), &mut o);
        assert_eq!(o[0].display.as_ref().unwrap().rounded, 0);
        assert_eq!(o[1].display.as_ref().unwrap().rounded, 100);
        assert_eq!(o[0].kind, "risk");
        assert_eq!(o[1].kind, "risk");
        assert_eq!(o[0].display.as_ref().unwrap().highlight, "best");
        assert_eq!(o[1].display.as_ref().unwrap().highlight, "worst");
    }
    #[test]
    fn identical_odds_are_not_arbitrarily_highlighted() {
        let mut o = odds(&[0.2; 12]);
        apply(&board(12), &mut o);
        assert!(
            o.iter()
                .all(|o| o.display.as_ref().unwrap().highlight == "none"
                    && o.display.as_ref().unwrap().muted)
        );
    }
    #[test]
    fn color_scale_has_green_orange_red_endpoints() {
        assert_eq!(risk_color(0.0), "#36f9a3");
        assert_eq!(risk_color(0.5), "#ff9a24");
        assert_eq!(risk_color(1.0), "#ff3038");
    }
}
