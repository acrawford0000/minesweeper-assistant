//! Line-oriented harness for exercising the real Rust engine in UI integration tests.
use minesweeper_assistant_lib::{example_board as demo_board, solver, vision};
use std::{
    io::{self, BufRead},
    sync::{Arc, atomic::AtomicU64},
};
fn main() {
    for line in io::stdin().lock().lines() {
        let result = (|| -> Result<serde_json::Value, String> {
            let input: serde_json::Value = serde_json::from_str(&line.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            match input["command"].as_str().unwrap_or("") {
                "analyze" => {
                    let board = serde_json::from_value(input["args"]["board"].clone())
                        .map_err(|e| e.to_string())?;
                    Ok(
                        serde_json::to_value(solver::solve(board, Arc::new(AtomicU64::new(0)), 0))
                            .unwrap(),
                    )
                }
                "scan_image" => {
                    let request = serde_json::from_value(input["args"]["request"].clone())
                        .map_err(|e| e.to_string())?;
                    Ok(serde_json::to_value(vision::scan(request)?).unwrap())
                }
                "demo_board" => Ok(serde_json::to_value(demo_board()).unwrap()),
                "compare_boards" => {
                    let previous = serde_json::from_value(input["args"]["previous"].clone())
                        .map_err(|e| e.to_string())?;
                    let current = serde_json::from_value(input["args"]["current"].clone())
                        .map_err(|e| e.to_string())?;
                    Ok(
                        serde_json::to_value(minesweeper_assistant_lib::board_progress::compare(
                            &previous, &current,
                        )?)
                        .unwrap(),
                    )
                }
                "cancel_analysis" => Ok(serde_json::Value::Null),
                "mine_total_from_counter" => {
                    let board: solver::Board =
                        serde_json::from_value(input["args"]["board"].clone())
                            .map_err(|e| e.to_string())?;
                    solver::validate(&board)?;
                    let remaining = input["args"]["remaining"]
                        .as_i64()
                        .ok_or("Missing counter value")?;
                    Ok(serde_json::json!(solver::total_from_counter(
                        i32::try_from(remaining).map_err(|e| e.to_string())?,
                        &board.cells
                    )?))
                }
                _ => Err("Unsupported test command".into()),
            }
        })();
        println!(
            "{}",
            match result {
                Ok(value) => serde_json::json!({"ok":value}),
                Err(error) => serde_json::json!({"error":error}),
            }
        );
    }
}
