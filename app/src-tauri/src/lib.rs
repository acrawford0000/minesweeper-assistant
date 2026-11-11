pub mod board_progress;
pub mod efficiency;
pub mod risk_display;
pub mod screen_capture;
pub mod solver;
pub mod vision;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tauri::Emitter;

#[derive(Default)]
struct Control(Arc<AtomicU64>);

#[tauri::command]
async fn analyze(
    board: solver::Board,
    state: tauri::State<'_, Control>,
) -> Result<solver::Analysis, String> {
    solver::validate(&board)?;
    let control = state.0.clone();
    let ticket = control.fetch_add(1, Ordering::Relaxed) + 1;
    tauri::async_runtime::spawn_blocking(move || solver::solve(board, control, ticket))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_analysis(state: tauri::State<'_, Control>) {
    state.0.fetch_add(1, Ordering::Relaxed);
}

#[tauri::command]
async fn scan_image(request: vision::ScanRequest) -> Result<vision::ScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || vision::scan(request))
        .await
        .map_err(|e| e.to_string())?
}

pub fn example_board() -> solver::Board {
    let rows = 9;
    let cols = 9;
    let mines = [6, 12, 21, 29, 34, 47, 55, 64, 70, 78];
    let mut cells = vec![-1; rows * cols];
    for i in 0..cells.len() {
        let r = i / cols;
        let c = i % cols;
        if !mines.contains(&i) && ((c <= 3 && r <= 5) || (r >= 6 && c >= 6)) {
            cells[i] = solver::neighbors(i, rows, cols)
                .iter()
                .filter(|v| mines.contains(v))
                .count() as i8;
        }
    }
    solver::Board {
        rows,
        cols,
        cells,
        total_mines: Some(10),
        trust_flags: true,
        revision: 0,
    }
}

#[tauri::command]
fn demo_board() -> solver::Board {
    example_board()
}

#[tauri::command]
fn mine_total_from_counter(board: solver::Board, remaining: i32) -> Result<usize, String> {
    solver::validate(&board)?;
    solver::total_from_counter(remaining, &board.cells)
}

#[tauri::command]
fn compare_boards(
    previous: solver::Board,
    current: solver::Board,
) -> Result<board_progress::Comparison, String> {
    board_progress::compare(&previous, &current)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Control::default())
        .manage(screen_capture::CaptureState::default())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let _ = app.emit_to("main", "capture-hotkey", ());
                    }
                })
                .build(),
        )
        .setup(|app| {
            screen_capture::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            analyze,
            cancel_analysis,
            scan_image,
            demo_board,
            mine_total_from_counter,
            compare_boards,
            screen_capture::capture_status,
            screen_capture::select_capture_area,
            screen_capture::capture_preview,
            screen_capture::finish_capture_selection,
            screen_capture::cancel_capture_selection,
            screen_capture::capture_region,
            screen_capture::set_capture_hotkey,
            screen_capture::set_capture_hotkey_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running Minesweeper Assistant");
}
