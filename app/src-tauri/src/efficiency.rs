//! Click planning for the currently provable frontier, including shared flag costs.
use crate::solver::{Board, COVERED, FLAG, MINE, Odds, neighbors};
use serde::Serialize;
use std::{
    collections::{BTreeSet, HashMap},
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub kind: String,
    pub cell: usize,
    pub clears: Vec<usize>,
    pub explanation: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub status: String,
    pub actions: Vec<Action>,
    pub targets: Vec<usize>,
    pub clicks: usize,
    pub known_safe: usize,
    pub projected_clicks: usize,
    pub direct_clicks: usize,
    pub optimal: bool,
    pub message: String,
}
#[derive(Clone)]
struct Candidate {
    kind: &'static str,
    cell: usize,
    covers: Vec<usize>,
    flags: Vec<usize>,
}

fn gain(c: &Candidate, cleared: &[bool]) -> usize {
    c.covers.iter().filter(|&&v| !cleared[v]).count()
}
fn new_flags(c: &Candidate, flags: &[bool]) -> usize {
    c.flags.iter().filter(|&&v| !flags[v]).count()
}
fn choose_greedy(candidates: &[Candidate], safe: usize, n: usize) -> (Vec<usize>, usize) {
    let mut cleared = vec![false; safe];
    let mut flags = vec![false; n];
    let mut result = Vec::new();
    let mut cost = 0;
    let start = Instant::now();
    while cleared.iter().any(|v| !*v) {
        let best = (0..candidates.len())
            .filter(|&i| gain(&candidates[i], &cleared) > 0)
            .max_by(|&a, &b| {
                let ga = gain(&candidates[a], &cleared);
                let gb = gain(&candidates[b], &cleared);
                let ca = 1 + new_flags(&candidates[a], &flags);
                let cb = 1 + new_flags(&candidates[b], &flags);
                (ga * cb)
                    .cmp(&(gb * ca))
                    .then_with(|| cb.cmp(&ca))
                    .then_with(|| ga.cmp(&gb))
                    .then_with(|| b.cmp(&a))
            })
            .unwrap();
        let mut selected = vec![best];
        let mut best_gain = gain(&candidates[best], &cleared);
        let mut best_cost = 1 + new_flags(&candidates[best], &flags);
        if start.elapsed() < Duration::from_millis(150) {
            let mut setup: Vec<_> = (0..candidates.len())
                .filter(|&i| {
                    candidates[i].kind == "chord"
                        && !candidates[i].flags.is_empty()
                        && gain(&candidates[i], &cleared) > 0
                })
                .collect();
            setup.sort_by_key(|&i| {
                (
                    std::cmp::Reverse(gain(&candidates[i], &cleared)),
                    new_flags(&candidates[i], &flags),
                )
            });
            setup.truncate(32);
            for (position, &a) in setup.iter().enumerate() {
                for &b in &setup[position + 1..] {
                    let union: BTreeSet<_> = candidates[a]
                        .covers
                        .iter()
                        .chain(&candidates[b].covers)
                        .copied()
                        .filter(|&v| !cleared[v])
                        .collect();
                    let shared: BTreeSet<_> = candidates[a]
                        .flags
                        .iter()
                        .chain(&candidates[b].flags)
                        .copied()
                        .filter(|&v| !flags[v])
                        .collect();
                    let pair_cost = 2 + shared.len();
                    if union.len() * best_cost > best_gain * pair_cost {
                        best_gain = union.len();
                        best_cost = pair_cost;
                        selected = vec![a, b];
                    }
                }
            }
        }
        for index in selected {
            let c = &candidates[index];
            if gain(c, &cleared) == 0 {
                continue;
            }
            cost += 1 + new_flags(c, &flags);
            for &v in &c.flags {
                flags[v] = true;
            }
            for &v in &c.covers {
                cleared[v] = true;
            }
            result.push(index);
        }
    }
    (result, cost)
}

struct Exact<'a, F: Fn() -> bool> {
    candidates: &'a [Candidate],
    covers: Vec<u64>,
    flags: Vec<u64>,
    all: u64,
    best: Vec<usize>,
    best_cost: usize,
    best_flags: u32,
    memo: HashMap<(u64, u64), usize>,
    nodes: usize,
    start: Instant,
    stopped: bool,
    cancelled: F,
}
impl<F: Fn() -> bool> Exact<'_, F> {
    fn visit(&mut self, covered: u64, flags: u64, cost: usize, path: &mut Vec<usize>) {
        self.nodes += 1;
        if self.nodes % 128 == 0
            && (self.nodes > 100_000
                || self.start.elapsed() > Duration::from_millis(200)
                || (self.cancelled)())
        {
            self.stopped = true;
            return;
        }
        if covered == self.all {
            if cost < self.best_cost
                || (cost == self.best_cost && flags.count_ones() < self.best_flags)
            {
                self.best = path.clone();
                self.best_cost = cost;
                self.best_flags = flags.count_ones();
            }
            return;
        }
        if cost >= self.best_cost {
            return;
        }
        if self
            .memo
            .get(&(covered, flags))
            .is_some_and(|&previous| previous <= cost)
        {
            return;
        }
        self.memo.insert((covered, flags), cost);
        let missing = self.all & !covered;
        let target = (0..64)
            .filter(|&v| missing & (1u64 << v) != 0)
            .min_by_key(|&v| {
                self.covers
                    .iter()
                    .filter(|&&c| c & (1u64 << v) != 0)
                    .count()
            })
            .unwrap();
        let mut options: Vec<_> = (0..self.candidates.len())
            .filter(|&i| self.covers[i] & (1u64 << target) != 0)
            .collect();
        options.sort_by_key(|&i| {
            (
                1 + (self.flags[i] & !flags).count_ones(),
                std::cmp::Reverse((self.covers[i] & missing).count_ones()),
            )
        });
        for i in options {
            let next_cost = cost + 1 + (self.flags[i] & !flags).count_ones() as usize;
            if next_cost > self.best_cost {
                continue;
            }
            path.push(i);
            self.visit(
                covered | self.covers[i],
                flags | self.flags[i],
                next_cost,
                path,
            );
            path.pop();
            if self.stopped {
                return;
            }
        }
    }
}

fn optimize<F: Fn() -> bool>(
    candidates: &[Candidate],
    safe: usize,
    n: usize,
    cancelled: F,
) -> (Vec<usize>, usize, bool) {
    let (baseline, cost) = choose_greedy(candidates, safe, n);
    let flag_ids: Vec<_> = candidates
        .iter()
        .flat_map(|c| c.flags.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if safe > 24 || flag_ids.len() > 20 || candidates.len() > 96 {
        return (baseline, cost, false);
    }
    let covers: Vec<_> = candidates
        .iter()
        .map(|c| c.covers.iter().fold(0u64, |mask, &v| mask | (1 << v)))
        .collect();
    let flags: Vec<_> = candidates
        .iter()
        .map(|c| {
            c.flags.iter().fold(0u64, |mask, v| {
                mask | (1 << flag_ids.binary_search(v).unwrap())
            })
        })
        .collect();
    let best_flags = baseline
        .iter()
        .fold(0u64, |mask, &i| mask | flags[i])
        .count_ones();
    let mut search = Exact {
        candidates,
        covers,
        flags,
        all: (1 << safe) - 1,
        best: baseline,
        best_cost: cost,
        best_flags,
        memo: HashMap::new(),
        nodes: 0,
        start: Instant::now(),
        stopped: false,
        cancelled,
    };
    search.visit(0, 0, 0, &mut Vec::new());
    (search.best, search.best_cost, !search.stopped)
}

pub fn plan<F: Fn() -> bool>(
    board: &Board,
    odds: &[Odds],
    recommendations: &[usize],
    cancelled: F,
) -> Plan {
    let safe: Vec<_> = board
        .cells
        .iter()
        .enumerate()
        .filter(|(i, v)| **v == COVERED && odds[*i].kind == "safe")
        .map(|(i, _)| i)
        .collect();
    let mut out = Plan {
        status: "safe".into(),
        actions: Vec::new(),
        targets: Vec::new(),
        clicks: 0,
        known_safe: safe.len(),
        projected_clicks: 0,
        direct_clicks: safe.len(),
        optimal: false,
        message: String::new(),
    };
    if safe.is_empty() {
        if let Some(&cell) = recommendations.first() {
            out.status = "guess".into();
            out.actions.push(Action{kind:"reveal".into(),cell,clears:vec![cell],explanation:"No provably safe reveal is available. This is the lowest-risk computed guess, not a guaranteed efficiency gain.".into()});
            out.clicks = 1;
            out.targets = vec![cell];
            out.message =
                "A guess is required. Future click cost is unknown until the result is visible."
                    .into();
        } else if board
            .cells
            .iter()
            .enumerate()
            .all(|(i, &v)| v != COVERED || odds[i].kind == "mine")
        {
            out.status = "done".into();
            out.message =
                "No safe squares remain to open. Extra flags do not help complete the board."
                    .into();
        } else {
            out.status = "unavailable".into();
            out.message="No safe sequence is currently available. Check the board or use Odds mode to inspect incomplete analysis.".into();
        }
        return out;
    }
    let n = board.cells.len();
    let mut local = vec![usize::MAX; n];
    for (i, &cell) in safe.iter().enumerate() {
        local[cell] = i;
    }
    let is_safe = |i: usize| board.cells[i] >= 0 || odds[i].kind == "safe";
    let zero: Vec<_> = (0..n)
        .map(|i| {
            local[i] != usize::MAX
                && neighbors(i, board.rows, board.cols)
                    .iter()
                    .all(|&v| is_safe(v))
        })
        .collect();
    let mut candidates = Vec::new();
    let mut seen = vec![false; n];
    for &cell in &safe {
        if zero[cell] {
            if seen[cell] {
                continue;
            }
            let mut queue = vec![cell];
            seen[cell] = true;
            let mut opening = BTreeSet::new();
            while let Some(v) = queue.pop() {
                opening.insert(local[v]);
                for u in neighbors(v, board.rows, board.cols) {
                    if local[u] != usize::MAX {
                        opening.insert(local[u]);
                        if zero[u] && !seen[u] {
                            seen[u] = true;
                            queue.push(u);
                        }
                    }
                }
            }
            candidates.push(Candidate {
                kind: "reveal",
                cell,
                covers: opening.into_iter().collect(),
                flags: Vec::new(),
            });
        } else {
            candidates.push(Candidate {
                kind: "reveal",
                cell,
                covers: vec![local[cell]],
                flags: Vec::new(),
            });
        }
    }
    for (cell, &clue) in board.cells.iter().enumerate().filter(|(_, v)| **v >= 0) {
        let around = neighbors(cell, board.rows, board.cols);
        let existing = around
            .iter()
            .filter(|&&v| board.cells[v] == FLAG || board.cells[v] == MINE)
            .count();
        let flags: Vec<_> = around
            .iter()
            .copied()
            .filter(|&v| board.cells[v] == COVERED && odds[v].kind == "mine")
            .collect();
        let unknown = around.iter().any(|&v| {
            board.cells[v] == COVERED && odds[v].kind != "safe" && odds[v].kind != "mine"
        });
        let covers: Vec<_> = around
            .iter()
            .copied()
            .filter(|&v| local[v] != usize::MAX)
            .map(|v| local[v])
            .collect();
        if !unknown && !covers.is_empty() && existing + flags.len() == clue as usize {
            candidates.push(Candidate {
                kind: "chord",
                cell,
                covers,
                flags,
            });
        }
    }
    let (chosen, cost, optimal) = optimize(&candidates, safe.len(), n, &cancelled);
    // Action ordering does not change total setup cost. Start with the best
    // immediate reveal, and defer flags until the chord that actually needs them.
    let first = *chosen
        .iter()
        .max_by(|&&a, &&b| {
            let ca = 1 + candidates[a].flags.len();
            let cb = 1 + candidates[b].flags.len();
            (candidates[a].covers.len() * cb)
                .cmp(&(candidates[b].covers.len() * ca))
                .then_with(|| cb.cmp(&ca))
                .then_with(|| b.cmp(&a))
        })
        .unwrap();
    let action = &candidates[first];
    for &cell in &action.flags {
        out.actions.push(Action {
            kind: "flag".into(),
            cell,
            clears: Vec::new(),
            explanation: "This cell is a proven mine. Flag it to enable the following chord."
                .into(),
        });
    }
    let clears: Vec<_> = action.covers.iter().map(|&i| safe[i]).collect();
    out.actions.push(Action{kind:action.kind.into(),cell:action.cell,clears:clears.clone(),explanation:if action.kind=="chord"{format!("After the listed flags, this clue has its required flags and opens at least {} known-safe cells.",clears.len())}else if clears.len()>1{format!("This is a proven zero opening, revealing at least {} known-safe cells with one click.",clears.len())}else{"Reveal this proven-safe cell directly; a flag/chord setup does not reduce the selected plan's click count.".into()}});
    out.clicks = out.actions.len();
    out.targets = clears;
    out.projected_clicks = cost;
    out.optimal = optimal;
    out.message = format!(
        "{} current-known-safe plan: {cost} clicks versus {} individual reveals. Flags count as clicks and can be shared by chords. Follow this prefix, then paste a new screenshot before continuing; hidden openings may change the next plan.",
        if optimal {
            "Minimum-click"
        } else {
            "Best-found"
        },
        safe.len()
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn c(covers: &[usize], flags: &[usize]) -> Candidate {
        Candidate {
            kind: "chord",
            cell: 0,
            covers: covers.to_vec(),
            flags: flags.to_vec(),
        }
    }
    #[test]
    fn avoids_flag_when_direct_reveal_is_cheaper() {
        let cs = vec![c(&[0], &[]), c(&[0], &[4])];
        let (p, cost, exact) = optimize(&cs, 1, 5, || false);
        assert!(exact);
        assert_eq!(cost, 1);
        assert_eq!(p, vec![0]);
    }
    #[test]
    fn shared_flag_can_make_two_chords_cheaper() {
        let cs = vec![
            c(&[0], &[]),
            c(&[1], &[]),
            c(&[2], &[]),
            c(&[3], &[]),
            c(&[0, 1], &[4]),
            c(&[2, 3], &[4]),
        ];
        let (p, cost, exact) = optimize(&cs, 4, 5, || false);
        assert!(exact);
        assert_eq!(cost, 3);
        assert!(p.contains(&4) && p.contains(&5));
    }
    #[test]
    fn large_frontier_heuristic_also_reuses_flags() {
        let mut cs: Vec<_> = (0..26).map(|i| c(&[i], &[])).collect();
        cs.push(c(&[0, 1], &[26]));
        cs.push(c(&[2, 3], &[26]));
        let (p, cost, exact) = optimize(&cs, 26, 27, || false);
        assert!(!exact);
        assert_eq!(cost, 25);
        assert!(p.contains(&26) && p.contains(&27));
    }
    #[test]
    fn equal_cost_prefers_no_new_flags() {
        let cs = vec![c(&[0], &[]), c(&[1], &[]), c(&[0, 1], &[2])];
        let (p, cost, _) = optimize(&cs, 2, 3, || false);
        assert_eq!(cost, 2);
        assert!(p.iter().all(|&i| cs[i].flags.is_empty()));
    }
    #[test]
    fn known_opening_beats_flag_chord() {
        let cs = vec![c(&[0, 1, 2], &[]), c(&[0, 1, 2], &[3])];
        let (p, cost, _) = optimize(&cs, 3, 4, || false);
        assert_eq!((p, cost), (vec![0], 1));
    }
    #[test]
    fn flag_and_chord_are_ordered_before_any_new_information() {
        let board = Board {
            rows: 3,
            cols: 3,
            cells: vec![-1, -1, -1, -1, 1, -1, -1, -1, -1],
            total_mines: Some(1),
            trust_flags: true,
            revision: 0,
        };
        let odds: Vec<_> = (0..9)
            .map(|i| Odds {
                probability: Some(if i == 1 { 1.0 } else { 0.0 }),
                kind: if i == 1 { "mine" } else { "safe" }.into(),
                reason: String::new(),
                numerator: None,
                display: None,
            })
            .collect();
        let result = plan(&board, &odds, &[], || false);
        assert!(result.optimal);
        assert_eq!(result.projected_clicks, 2);
        assert_eq!(result.actions.len(), 2);
        assert_eq!(
            (result.actions[0].kind.as_str(), result.actions[0].cell),
            ("flag", 1)
        );
        assert_eq!(
            (result.actions[1].kind.as_str(), result.actions[1].cell),
            ("chord", 4)
        );
        assert_eq!(result.targets.len(), 7);
    }
    #[test]
    fn guess_and_finished_board_do_not_add_unnecessary_flags() {
        use std::sync::{Arc, atomic::AtomicU64};
        let b = Board {
            rows: 1,
            cols: 2,
            cells: vec![-1, -1],
            total_mines: Some(1),
            trust_flags: true,
            revision: 0,
        };
        let a = crate::solver::solve(b.clone(), Arc::new(AtomicU64::new(0)), 0);
        let p = a.efficiency.unwrap();
        assert_eq!(p.status, "guess");
        assert_eq!(p.actions.len(), 1);
        assert_eq!(p.actions[0].kind, "reveal");
        let a = crate::solver::solve(
            Board {
                total_mines: Some(2),
                ..b
            },
            Arc::new(AtomicU64::new(0)),
            0,
        );
        assert_eq!(a.efficiency.unwrap().status, "done");
    }
    #[test]
    fn sequence_is_legal_and_safe_in_the_real_example_position() {
        use std::sync::{Arc, atomic::AtomicU64};
        let board = crate::example_board();
        let result = crate::solver::solve(board.clone(), Arc::new(AtomicU64::new(0)), 0);
        let plan = result.efficiency.unwrap();
        assert_eq!(plan.status, "safe");
        let mut cells = board.cells.clone();
        for action in &plan.actions {
            match action.kind.as_str() {
                "flag" => {
                    assert_eq!(result.odds[action.cell].kind, "mine");
                    assert_eq!(cells[action.cell], COVERED);
                    cells[action.cell] = FLAG;
                }
                "chord" => {
                    let around = neighbors(action.cell, board.rows, board.cols);
                    assert_eq!(
                        around
                            .iter()
                            .filter(|&&i| cells[i] == FLAG || cells[i] == MINE)
                            .count(),
                        cells[action.cell] as usize
                    );
                    assert!(
                        around
                            .iter()
                            .all(|&i| cells[i] != COVERED || result.odds[i].kind == "safe")
                    );
                }
                "reveal" => assert_eq!(result.odds[action.cell].kind, "safe"),
                _ => panic!("unknown action"),
            }
        }
        assert_eq!(plan.actions.iter().filter(|a| a.kind != "flag").count(), 1);
    }
}
