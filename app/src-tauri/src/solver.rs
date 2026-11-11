use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

pub const COVERED: i8 = -1;
pub const FLAG: i8 = -2;
pub const MINE: i8 = -3;
pub const SAFE: i8 = -4;
pub const UNREADABLE: i8 = -5;

pub fn total_from_counter(remaining: i32, cells: &[i8]) -> Result<usize, String> {
    let flags = cells.iter().filter(|&&v| v == FLAG || v == MINE).count();
    let total = remaining as i64 + flags as i64;
    if total < 0 || total > cells.len() as i64 {
        return Err(
            "The remaining-flags counter and placed flags give an invalid mine total.".into(),
        );
    }
    Ok(total as usize)
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub rows: usize,
    pub cols: usize,
    pub cells: Vec<i8>,
    pub total_mines: Option<usize>,
    #[serde(default)]
    pub trust_flags: bool,
    #[serde(default)]
    pub revision: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Odds {
    pub probability: Option<f64>,
    pub kind: String,
    pub reason: String,
    pub numerator: Option<String>,
    pub display: Option<crate::risk_display::RiskDisplay>,
}

impl Odds {
    fn unknown() -> Self {
        Self {
            probability: None,
            kind: "unknown".into(),
            reason: String::new(),
            numerator: None,
            display: None,
        }
    }
    fn fixed(mine: bool, reason: &str) -> Self {
        Self {
            probability: Some(if mine { 1.0 } else { 0.0 }),
            kind: if mine { "mine" } else { "safe" }.into(),
            reason: reason.into(),
            numerator: None,
            display: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub revision: u64,
    pub status: String,
    pub message: String,
    pub odds: Vec<Odds>,
    pub solutions: Option<String>,
    pub conflicts: Vec<usize>,
    pub nodes: u64,
    pub elapsed_ms: u128,
    pub components: usize,
    pub cached_components: usize,
    pub pooled_cells: usize,
    pub recommendations: Vec<usize>,
    pub efficiency: Option<crate::efficiency::Plan>,
}

#[derive(Clone, Debug)]
struct Constraint {
    cells: Vec<usize>,
    target: i32,
    clues: Vec<usize>,
}

pub fn neighbors(index: usize, rows: usize, cols: usize) -> Vec<usize> {
    let r = index / cols;
    let c = index % cols;
    let mut result = Vec::with_capacity(8);
    for y in r.saturating_sub(1)..=(r + 1).min(rows - 1) {
        for x in c.saturating_sub(1)..=(c + 1).min(cols - 1) {
            if y != r || x != c {
                result.push(y * cols + x);
            }
        }
    }
    result
}

pub fn validate(board: &Board) -> Result<(), String> {
    if board.rows == 0
        || board.cols == 0
        || board.rows > 100
        || board.cols > 100
        || board.rows * board.cols > 2500
    {
        return Err("Use 1–100 rows and columns, with at most 2,500 cells.".into());
    }
    if board.cells.len() != board.rows * board.cols
        || board.cells.iter().any(|v| *v < UNREADABLE || *v > 8)
    {
        return Err("The board contains invalid cell data.".into());
    }
    if board.total_mines.is_some_and(|n| n > board.cells.len()) {
        return Err("The mine total exceeds the number of cells.".into());
    }
    Ok(())
}

fn reduce(
    constraints: &[Constraint],
    fixed: &[Option<bool>],
) -> Result<Vec<Constraint>, Vec<usize>> {
    let mut result: Vec<Constraint> = Vec::new();
    for eq in constraints {
        let mut target = eq.target;
        let cells: Vec<_> = eq
            .cells
            .iter()
            .copied()
            .filter(|&i| {
                if fixed[i] == Some(true) {
                    target -= 1;
                }
                fixed[i].is_none()
            })
            .collect();
        if target < 0 || target as usize > cells.len() {
            return Err(eq.clues.clone());
        }
        if cells.is_empty() {
            continue;
        }
        if let Some(previous) = result.iter().find(|e| e.cells == cells) {
            if previous.target != target {
                let mut conflicts = previous.clues.clone();
                conflicts.extend(&eq.clues);
                return Err(conflicts);
            }
        } else {
            result.push(Constraint {
                cells,
                target,
                clues: eq.clues.clone(),
            });
        }
    }
    Ok(result)
}

struct Budget {
    start: Instant,
    nodes: u64,
    control: Arc<AtomicU64>,
    ticket: u64,
}
impl Budget {
    fn stop(&self) -> bool {
        self.nodes > 2_000_000
            || self.start.elapsed() > Duration::from_secs(3)
            || self.control.load(Ordering::Relaxed) != self.ticket
    }
}

struct Counts {
    cells: Vec<usize>,
    totals: Vec<BigUint>,
    mines: Vec<Vec<BigUint>>,
}

type ComponentKey = Vec<(Vec<usize>, i32)>;
#[derive(Default)]
struct ComponentCache(VecDeque<(ComponentKey, Arc<Counts>)>);
static COMPONENT_CACHE: OnceLock<Mutex<ComponentCache>> = OnceLock::new();

fn count_component(
    cells: &[usize],
    equations: &[Constraint],
    budget: &mut Budget,
    cache: &Mutex<ComponentCache>,
) -> Option<(Arc<Counts>, bool)> {
    let mut key: ComponentKey = equations
        .iter()
        .map(|eq| {
            let mut cells = eq.cells.clone();
            cells.sort_unstable();
            (cells, eq.target)
        })
        .collect();
    key.sort_unstable();
    if let Some(counts) = cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .0
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.clone())
    {
        return Some((counts, true));
    }
    // Only complete, exact histograms enter the cache. Global mine weights are
    // applied afresh, so a counter edit can safely reuse unchanged frontiers.
    let counts = Arc::new(enumerate(cells, equations, budget)?);
    let mut saved = cache.lock().unwrap_or_else(|e| e.into_inner());
    if saved.0.len() >= 32 {
        saved.0.pop_front();
    }
    saved.0.push_back((key, counts.clone()));
    Some((counts, false))
}

fn enumerate(cells: &[usize], equations: &[Constraint], budget: &mut Budget) -> Option<Counts> {
    if cells.len() > 60 {
        return None;
    }
    let mut order = cells.to_vec();
    order.sort_by_key(|v| {
        std::cmp::Reverse(equations.iter().filter(|e| e.cells.contains(v)).count())
    });
    let local: Vec<(Vec<usize>, i32)> = equations
        .iter()
        .map(|e| {
            (
                e.cells
                    .iter()
                    .map(|v| order.iter().position(|x| x == v).unwrap())
                    .collect(),
                e.target,
            )
        })
        .collect();
    let n = order.len();
    let mut counts = Counts {
        cells: order,
        totals: vec![BigUint::zero(); n + 1],
        mines: vec![vec![BigUint::zero(); n + 1]; n],
    };
    let mut values = vec![-1i8; n];
    fn visit(
        pos: usize,
        mine_count: usize,
        values: &mut [i8],
        eqs: &[(Vec<usize>, i32)],
        counts: &mut Counts,
        budget: &mut Budget,
    ) -> bool {
        budget.nodes += 1;
        if budget.nodes % 512 == 0 && budget.stop() {
            return false;
        }
        for (vars, target) in eqs {
            let mut sum = 0;
            let mut free = 0;
            for &v in vars {
                if values[v] < 0 {
                    free += 1;
                } else {
                    sum += values[v] as i32;
                }
            }
            if sum > *target || sum + free < *target {
                return true;
            }
        }
        if pos == values.len() {
            counts.totals[mine_count] += 1u32;
            for (v, &value) in values.iter().enumerate() {
                if value == 1 {
                    counts.mines[v][mine_count] += 1u32;
                }
            }
            return true;
        }
        for value in 0..=1 {
            values[pos] = value;
            if !visit(
                pos + 1,
                mine_count + value as usize,
                values,
                eqs,
                counts,
                budget,
            ) {
                return false;
            }
        }
        values[pos] = -1;
        true
    }
    if !visit(0, 0, &mut values, &local, &mut counts, budget) {
        None
    } else {
        Some(counts)
    }
}

fn convolve(a: &[BigUint], b: &[BigUint], cap: usize) -> Vec<BigUint> {
    let mut result = vec![BigUint::zero(); (a.len() + b.len() - 1).min(cap + 1)];
    for (i, x) in a.iter().enumerate() {
        if x.is_zero() {
            continue;
        }
        for (j, y) in b.iter().enumerate() {
            if i + j >= result.len() {
                break;
            }
            if !y.is_zero() {
                result[i + j] += x * y;
            }
        }
    }
    result
}

fn binomials(n: usize, cap: usize) -> Vec<BigUint> {
    let mut values = vec![BigUint::one()];
    for k in 1..=n.min(cap) {
        values.push(&values[k - 1] * (n - k + 1) / k);
    }
    values
}

fn weighted(hist: &[BigUint], choose: &[BigUint], remaining: usize) -> BigUint {
    hist.iter()
        .enumerate()
        .filter_map(|(k, count)| {
            remaining
                .checked_sub(k)
                .and_then(|r| choose.get(r))
                .map(|weight| count * weight)
        })
        .sum()
}

fn fraction(a: &BigUint, b: &BigUint) -> f64 {
    let shift = b.bits().saturating_sub(900) as usize;
    (a >> shift).to_f64().unwrap_or(0.0) / (b >> shift).to_f64().unwrap_or(1.0)
}

pub fn solve(board: Board, control: Arc<AtomicU64>, ticket: u64) -> Analysis {
    let start = Instant::now();
    let mut budget = Budget {
        start,
        nodes: 0,
        control,
        ticket,
    };
    let mut out = Analysis {
        revision: board.revision,
        status: "invalid".into(),
        message: String::new(),
        odds: vec![Odds::unknown(); board.cells.len().min(2500)],
        solutions: None,
        conflicts: Vec::new(),
        nodes: 0,
        elapsed_ms: 0,
        components: 0,
        cached_components: 0,
        pooled_cells: 0,
        recommendations: Vec::new(),
        efficiency: None,
    };
    macro_rules! finish {
        ($status:expr, $message:expr) => {{
            out.status = $status.into();
            out.message = $message.into();
            out.nodes = budget.nodes;
            out.elapsed_ms = start.elapsed().as_millis();
            if out.status != "invalid" && out.status != "review" {
                out.recommendations = (0..out.odds.len())
                    .filter(|&i| {
                        (board.cells[i] == COVERED || board.cells[i] == FLAG)
                            && out.odds[i].probability.is_some_and(|p| p < 1.0)
                            && (out.status == "exact" || out.odds[i].kind == "safe")
                    })
                    .collect();
                out.recommendations.sort_by(|&a, &b| {
                    out.odds[a]
                        .probability
                        .unwrap()
                        .total_cmp(&out.odds[b].probability.unwrap())
                });
                crate::risk_display::apply(&board, &mut out.odds);
                out.efficiency = Some(crate::efficiency::plan(
                    &board,
                    &out.odds,
                    &out.recommendations,
                    || budget.control.load(Ordering::Relaxed) != budget.ticket,
                ));
                out.elapsed_ms = start.elapsed().as_millis();
            }
            return out;
        }};
    }
    if let Err(error) = validate(&board) {
        finish!("invalid", error);
    }
    if board.cells.contains(&UNREADABLE) {
        out.conflicts = board
            .cells
            .iter()
            .enumerate()
            .filter(|(_, v)| **v == UNREADABLE)
            .map(|(i, _)| i)
            .collect();
        finish!(
            "review",
            "Review the highlighted unreadable cells before calculating odds."
        );
    }
    let n = board.cells.len();
    let mut fixed = vec![None; n];
    for (i, value) in board.cells.iter().enumerate() {
        if *value >= 0 || *value == SAFE {
            fixed[i] = Some(false);
        }
        if *value == MINE || *value == FLAG {
            fixed[i] = Some(true);
        }
        if *value < 0 {
            if let Some(mine) = fixed[i] {
                out.odds[i] = Odds::fixed(
                    mine,
                    if *value == FLAG {
                        "Flagged cells are confirmed mines."
                    } else {
                        "Entered in the board."
                    },
                );
            }
        }
    }
    let mut equations: Vec<Constraint> = board
        .cells
        .iter()
        .enumerate()
        .filter(|(_, v)| **v >= 0)
        .map(|(i, v)| Constraint {
            cells: neighbors(i, board.rows, board.cols),
            target: *v as i32,
            clues: vec![i],
        })
        .collect();
    if let Some(total) = board.total_mines {
        equations.push(Constraint {
            cells: (0..n).collect(),
            target: total as i32,
            clues: Vec::new(),
        });
    }
    // The global equation participates in propagation, but is removed for component enumeration.
    loop {
        if budget.stop() {
            finish!(
                "limited",
                "Analysis was cancelled or reached its time limit. Only proven deductions are shown."
            );
        }
        equations = match reduce(&equations, &fixed) {
            Ok(e) => e,
            Err(conflicts) => {
                out.conflicts = conflicts;
                finish!(
                    "invalid",
                    "These clues, flags, or mine total cannot all be correct. Review the highlighted cells and mine count."
                );
            }
        };
        let mut changed = false;
        for eq in &equations {
            if eq.target == 0 || eq.target as usize == eq.cells.len() {
                let mine = eq.target != 0;
                let reason = if eq.clues.is_empty() {
                    "Forced by the remaining mine total.".to_owned()
                } else {
                    format!(
                        "Forced by clue{} {}.",
                        if eq.clues.len() > 1 { "s" } else { "" },
                        eq.clues
                            .iter()
                            .map(|i| format!("R{} C{}", i / board.cols + 1, i % board.cols + 1))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                for &i in &eq.cells {
                    if fixed[i].is_some_and(|old| old != mine) {
                        out.conflicts = eq.clues.clone();
                        finish!(
                            "invalid",
                            "Two deductions conflict. Check the clues and mine count."
                        );
                    }
                    if fixed[i].is_none() {
                        fixed[i] = Some(mine);
                        out.odds[i] = Odds::fixed(mine, &reason);
                        changed = true;
                    }
                }
            }
        }
        if changed {
            continue;
        }
        let mut added = Vec::new();
        // Bounded subset deduction prevents quadratic derived-constraint growth.
        if equations.len() <= 600 {
            for a in &equations {
                for b in &equations {
                    if a.cells.len() >= b.cells.len() || a.cells.len() > 16 {
                        continue;
                    }
                    if a.cells.iter().all(|v| b.cells.binary_search(v).is_ok()) {
                        let cells: Vec<_> = b
                            .cells
                            .iter()
                            .copied()
                            .filter(|v| a.cells.binary_search(v).is_err())
                            .collect();
                        let target = b.target - a.target;
                        if target < 0 || target as usize > cells.len() {
                            out.conflicts = a.clues.iter().chain(&b.clues).copied().collect();
                            finish!("invalid", "Overlapping clues contradict each other.");
                        }
                        if (target == 0 || target as usize == cells.len())
                            && !equations
                                .iter()
                                .chain(&added)
                                .any(|e| e.cells == cells && e.target == target)
                        {
                            let mut clues = a.clues.clone();
                            clues.extend(&b.clues);
                            clues.sort_unstable();
                            clues.dedup();
                            added.push(Constraint {
                                cells,
                                target,
                                clues,
                            });
                        }
                    }
                }
            }
        }
        if added.is_empty() {
            break;
        }
        equations.extend(added);
    }
    let Some(total) = board.total_mines else {
        finish!(
            "logic",
            "Logical deductions are ready. Enter total mines to calculate global percentages; flags are confirmed mines."
        );
    };
    let mines = fixed.iter().filter(|v| **v == Some(true)).count();
    let Some(remaining) = total.checked_sub(mines) else {
        finish!("invalid", "More confirmed mines than the declared total.");
    };
    // Rebuild local constraints to avoid connecting every component through the total.
    let original: Vec<_> = board
        .cells
        .iter()
        .enumerate()
        .filter(|(_, v)| **v >= 0)
        .map(|(i, v)| Constraint {
            cells: neighbors(i, board.rows, board.cols),
            target: *v as i32,
            clues: vec![i],
        })
        .collect();
    let local = match reduce(&original, &fixed) {
        Ok(e) => e,
        Err(c) => {
            out.conflicts = c;
            finish!("invalid", "Conflicting clues.");
        }
    };
    let mut links = vec![Vec::new(); n];
    for (e, eq) in local.iter().enumerate() {
        for &v in &eq.cells {
            links[v].push(e);
        }
    }
    let outside: Vec<_> = (0..n)
        .filter(|&v| fixed[v].is_none() && links[v].is_empty())
        .collect();
    let mut seen = vec![false; n];
    let mut groups = Vec::new();
    for root in 0..n {
        if fixed[root].is_some() || links[root].is_empty() || seen[root] {
            continue;
        }
        let mut queue = VecDeque::from([root]);
        let mut cells = Vec::new();
        let mut eqids = Vec::new();
        seen[root] = true;
        while let Some(v) = queue.pop_front() {
            cells.push(v);
            for &e in &links[v] {
                if !eqids.contains(&e) {
                    eqids.push(e);
                    for &u in &local[e].cells {
                        if !seen[u] {
                            seen[u] = true;
                            queue.push_back(u);
                        }
                    }
                }
            }
        }
        groups.push((
            cells,
            eqids.iter().map(|&e| local[e].clone()).collect::<Vec<_>>(),
        ));
    }
    out.components = groups.len();
    let mut components = Vec::new();
    for (cells, eqs) in groups {
        if budget.stop() {
            finish!(
                "limited",
                "Analysis was cancelled or reached its time limit. Only proven deductions are shown."
            );
        }
        match count_component(
            &cells,
            &eqs,
            &mut budget,
            COMPONENT_CACHE.get_or_init(|| Mutex::new(ComponentCache::default())),
        ) {
            Some((counts, cached)) => {
                out.cached_components += usize::from(cached);
                components.push(counts)
            }
            None => {
                finish!(
                    "limited",
                    "This frontier needs more search than the current budget allows. Proven moves remain available; uncomputed odds are left blank."
                );
            }
        }
    }
    let mut prefix = vec![vec![BigUint::one()]];
    for c in &components {
        if budget.stop() {
            finish!(
                "limited",
                "The global counting step reached the time limit. Only proven deductions are shown."
            );
        }
        prefix.push(convolve(prefix.last().unwrap(), &c.totals, remaining));
    }
    let mut suffix = vec![vec![BigUint::one()]; components.len() + 1];
    for j in (0..components.len()).rev() {
        suffix[j] = convolve(&components[j].totals, &suffix[j + 1], remaining);
    }
    let choose = binomials(outside.len(), remaining);
    let z = weighted(prefix.last().unwrap(), &choose, remaining);
    if z.is_zero() {
        finish!(
            "invalid",
            "No mine arrangements fit this board and total. Check clues, confirmed mines, and the mine count."
        );
    }
    out.solutions = Some(z.to_string());
    let set_odds = |numerator: BigUint, reason: &str| -> Odds {
        let kind = if numerator.is_zero() {
            "safe"
        } else if numerator == z {
            "mine"
        } else {
            "risk"
        };
        Odds {
            probability: Some(fraction(&numerator, &z)),
            kind: kind.into(),
            reason: reason.into(),
            numerator: Some(numerator.to_string()),
            display: None,
        }
    };
    for (j, c) in components.iter().enumerate() {
        if budget.stop() {
            finish!(
                "limited",
                "Global configurations were counted, but remaining cell percentages reached the time limit. Computed percentages are exact; blank cells are uncomputed."
            );
        }
        let others = convolve(&prefix[j], &suffix[j + 1], remaining);
        let mut equivalent: HashMap<Vec<BigUint>, Odds> = HashMap::new();
        for (v, &cell) in c.cells.iter().enumerate() {
            if let Some(odd) = equivalent.get(&c.mines[v]) {
                out.odds[cell] = odd.clone();
                continue;
            }
            let hist = convolve(&others, &c.mines[v], remaining);
            out.odds[cell] = set_odds(
                weighted(&hist, &choose, remaining),
                "Counted across every compatible board, including the global mine total.",
            );
            equivalent.insert(c.mines[v].clone(), out.odds[cell].clone());
        }
    }
    if !outside.is_empty() {
        // C(n-1,k) = C(n,k) * (n-k)/n; reuse the one shared pool's weights.
        let other_choose: Vec<_> = choose
            .iter()
            .enumerate()
            .map(|(k, value)| value * (outside.len() - k) / outside.len())
            .collect();
        let numerator = if remaining == 0 {
            BigUint::zero()
        } else {
            weighted(prefix.last().unwrap(), &other_choose, remaining - 1)
        };
        let shared = set_odds(
            numerator,
            "This cell belongs to the shared pool outside the clue frontier; one exact calculation supplies the odds for all cells in this pool.",
        );
        out.pooled_cells = outside.len();
        for cell in outside {
            out.odds[cell] = shared.clone();
        }
    }
    finish!(
        "exact",
        "Exact odds under the entered clues and total mine count. All compatible mine placements are weighted equally."
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_frontiers_reuse_only_matching_exact_constraints() {
        let cache = Mutex::new(ComponentCache::default());
        let mut budget = Budget {
            start: Instant::now(),
            nodes: 0,
            control: Arc::new(AtomicU64::new(0)),
            ticket: 0,
        };
        let eq = |target| {
            vec![Constraint {
                cells: vec![0, 1, 2],
                target,
                clues: vec![3],
            }]
        };
        let (first, hit) = count_component(&[0, 1, 2], &eq(1), &mut budget, &cache).unwrap();
        assert!(!hit);
        let nodes = budget.nodes;
        let (second, hit) = count_component(&[2, 1, 0], &eq(1), &mut budget, &cache).unwrap();
        assert!(hit);
        assert_eq!(nodes, budget.nodes);
        assert!(Arc::ptr_eq(&first, &second));
        let (changed, hit) = count_component(&[0, 1, 2], &eq(2), &mut budget, &cache).unwrap();
        assert!(!hit);
        assert_eq!(first.totals[1], BigUint::from(3u32));
        assert_eq!(changed.totals[1], BigUint::zero());
        assert_eq!(changed.totals[2], BigUint::from(3u32));
        let too_large: Vec<_> = (0..61).collect();
        assert!(
            count_component(
                &too_large,
                &[Constraint {
                    cells: too_large.clone(),
                    target: 30,
                    clues: vec![]
                }],
                &mut budget,
                &cache
            )
            .is_none()
        );
        assert_eq!(cache.lock().unwrap().0.len(), 2);
    }
    #[test]
    fn large_uniform_board_uses_one_exact_pool() {
        let a = run(50, 50, vec![COVERED; 2500], Some(550));
        assert_eq!(a.status, "exact");
        assert_eq!(a.nodes, 0);
        assert_eq!(a.pooled_cells, 2500);
        let first = &a.odds[0];
        assert!((first.probability.unwrap() - 0.22).abs() < 1e-12);
        assert!(
            a.odds
                .iter()
                .all(|o| o.numerator == first.numerator && o.probability == first.probability)
        );
        let capped = binomials(2500, 550);
        assert_eq!(capped.len(), 551);
        assert_eq!(binomials(4, 2), vec![1u32.into(), 4u32.into(), 6u32.into()]);
    }
    fn run(rows: usize, cols: usize, cells: Vec<i8>, total: Option<usize>) -> Analysis {
        solve(
            Board {
                rows,
                cols,
                cells,
                total_mines: total,
                trust_flags: false,
                revision: 7,
            },
            Arc::new(AtomicU64::new(0)),
            0,
        )
    }
    #[test]
    fn no_clues_has_density() {
        let a = run(2, 3, vec![-1; 6], Some(2));
        assert_eq!(a.solutions.as_deref(), Some("15"));
        for o in a.odds {
            assert!((o.probability.unwrap() - 1.0 / 3.0).abs() < 1e-12);
        }
    }
    #[test]
    fn flags_are_confirmed_mines() {
        let a = run(1, 3, vec![-2, 1, -1], Some(1));
        assert_eq!(a.odds[0].probability, Some(1.0));
        assert_eq!(a.odds[2].probability, Some(0.0));
    }
    #[test]
    fn remaining_flag_counter_adds_placed_flags_once() {
        assert_eq!(total_from_counter(3, &[-2, -2, -1, -1, -1, -1]), Ok(5));
        assert_eq!(total_from_counter(-1, &[-2, -2, -1]), Ok(1));
        assert!(total_from_counter(-3, &[-2, -1]).is_err());
    }
    #[test]
    fn contradictory_clue() {
        let a = run(1, 2, vec![2, -1], Some(1));
        assert_eq!(a.status, "invalid");
    }
    #[test]
    fn unknown_clue_is_not_zero() {
        let a = run(1, 3, vec![-4, -1, -1], Some(1));
        assert_eq!(a.odds[1].probability, Some(0.5));
    }
    #[test]
    fn unreadable_blocks() {
        assert_eq!(run(1, 2, vec![-5, -1], Some(1)).status, "review");
    }
    #[test]
    fn missing_total_still_proves_moves() {
        let a = run(1, 3, vec![0, -1, -1], None);
        assert_eq!(a.status, "logic");
        assert_eq!(a.odds[1].kind, "safe");
        assert_eq!(a.odds[2].probability, None);
    }
    #[test]
    fn zero_and_all_mines() {
        for total in [0, 6] {
            let a = run(2, 3, vec![-1; 6], Some(total));
            assert_eq!(a.status, "exact");
            for o in a.odds {
                assert_eq!(o.probability, Some(if total == 0 { 0.0 } else { 1.0 }));
            }
        }
    }
    #[test]
    fn disconnected_frontiers_and_outside_cell() {
        let a = run(1, 7, vec![-1, 1, -1, -1, -1, 1, -1], Some(3));
        assert_eq!(a.status, "exact");
        assert_eq!(a.solutions.as_deref(), Some("4"));
        for i in [0, 2, 4, 6] {
            assert_eq!(a.odds[i].probability, Some(0.5));
        }
        assert_eq!(a.odds[3].kind, "mine");
    }
    #[test]
    fn cancellation() {
        let a = solve(
            Board {
                rows: 2,
                cols: 2,
                cells: vec![-1; 4],
                total_mines: Some(1),
                trust_flags: false,
                revision: 1,
            },
            Arc::new(AtomicU64::new(2)),
            1,
        );
        assert_eq!(a.status, "limited");
    }
    #[test]
    fn cross_check_full_board_enumerator() {
        // Independent reference: test every entire 3x3 mine placement, without components or propagation.
        for seed in 0u32..120 {
            let actual = (seed.wrapping_mul(73) + 19) & 511;
            let total = actual.count_ones() as usize;
            let mut cells = vec![-1; 9];
            for i in 0..9 {
                if actual & (1 << i) == 0 && (seed as usize + i * 3) % 4 < 2 {
                    cells[i] = neighbors(i, 3, 3)
                        .iter()
                        .filter(|&&v| actual & (1 << v) != 0)
                        .count() as i8;
                }
            }
            let mut solutions = 0u64;
            let mut mines = [0u64; 9];
            for mask in 0u32..512 {
                if mask.count_ones() as usize != total {
                    continue;
                }
                let fits = (0..9).all(|i| {
                    cells[i] < 0
                        || (mask & (1 << i) == 0
                            && neighbors(i, 3, 3)
                                .iter()
                                .filter(|&&v| mask & (1 << v) != 0)
                                .count()
                                == cells[i] as usize)
                });
                if fits {
                    solutions += 1;
                    for (i, count) in mines.iter_mut().enumerate() {
                        if mask & (1 << i) != 0 {
                            *count += 1;
                        }
                    }
                }
            }
            let a = run(3, 3, cells.clone(), Some(total));
            assert_eq!(a.status, "exact", "seed {seed}");
            // Propagated facts remove fixed cells before counting, without changing number of boards.
            assert_eq!(
                a.solutions.as_ref().unwrap().parse::<u64>().unwrap(),
                solutions,
                "seed {seed}"
            );
            for i in 0..9 {
                if cells[i] < 0 {
                    assert!(
                        (a.odds[i].probability.unwrap() - mines[i] as f64 / solutions as f64).abs()
                            < 1e-12,
                        "seed {seed}, cell {i}"
                    );
                }
            }
        }
    }
}
