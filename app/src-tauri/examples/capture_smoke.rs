//! Native command smoke test: no mouse/keyboard injection and no files recorded.
use minesweeper_assistant_lib::screen_capture::{self as capture, CaptureState, Selection};
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
fn main() {
    tauri::Builder::default().manage(CaptureState::default()).plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![capture::capture_status,capture::capture_preview,capture::cancel_capture_selection])
        .setup(|app|{
            let handle=app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result:Result<(),String>=async{
                    tokio_pause().await;
                    let main=handle.get_webview_window("main").ok_or("Missing main window")?;
                    capture::init(&handle);let paused=capture::status(&handle);assert!(!paused.hotkey_enabled);assert_eq!(paused.configured_hotkey,"y");assert!(!handle.global_shortcut().is_registered("y"));
                    let initial=capture::configure_hotkey(&handle,Some(capture::DEFAULT_HOTKEY.into()))?;
                    assert!(initial.region.is_none());assert!(handle.global_shortcut().is_registered(capture::DEFAULT_HOTKEY));
                    println!("Smoke: select first area");capture::select_capture_area(handle.clone(),main.clone()).await?;
                    let overlay=handle.get_webview_window("capture-overlay").ok_or("Missing selector")?;
                    assert!(!main.is_visible().map_err(|e|e.to_string())?);
                    println!("Smoke: preview, active={}",capture::status(&handle).selecting);let preview=capture::capture_preview(handle.clone(),overlay.clone()).await?;assert!(preview.bounds.width>32&&preview.bounds.height>32);assert!(preview.image.starts_with("data:image/png;base64,"));
                    let selection=Selection{x:0.0,y:0.0,width:0.25,height:0.25};
                    let expected=capture::selected_rect(preview.bounds,&selection)?;
                    println!("Smoke: commit area, active={}",capture::status(&handle).selecting);capture::finish_capture_selection(handle.clone(),overlay,selection)?;
                    tokio_pause().await;assert!(main.is_visible().map_err(|e|e.to_string())?);
                    println!("Smoke: capture region");let frame=capture::capture_region(handle.clone(),main.clone()).await?;assert_eq!(frame.region,expected);assert!(frame.image.starts_with("data:image/png;base64,"));
                    capture::configure_hotkey(&handle,Some("Ctrl+Alt+F24".into()))?;assert!(handle.global_shortcut().is_registered("Ctrl+Alt+F24"));assert!(!handle.global_shortcut().is_registered(capture::DEFAULT_HOTKEY));
                    capture::configure_hotkey(&handle,None)?;assert!(!handle.global_shortcut().is_registered("Ctrl+Alt+F24"));
                    let paused=capture::status(&handle);assert_eq!(paused.configured_hotkey,"Ctrl+Alt+F24");assert!(!paused.hotkey_enabled);
                    capture::set_hotkey_enabled(&handle,true)?;assert!(handle.global_shortcut().is_registered("Ctrl+Alt+F24"));
                    capture::set_hotkey_enabled(&handle,false)?;
                    let changed=capture::change_hotkey_binding(&handle,"q".into())?;assert!(!changed.hotkey_enabled);assert_eq!(changed.configured_hotkey,"q");assert!(!handle.global_shortcut().is_registered("q"));
                    capture::set_hotkey_enabled(&handle,true)?;assert!(handle.global_shortcut().is_registered("q"));capture::set_hotkey_enabled(&handle,false)?;assert!(!handle.global_shortcut().is_registered("q"));assert_eq!(capture::status(&handle).region,Some(expected));
                    println!("Smoke: select second area");capture::select_capture_area(handle.clone(),main.clone()).await?;
                    let overlay=handle.get_webview_window("capture-overlay").ok_or("Missing second selector")?;
                    capture::cancel_capture_selection(handle.clone(),overlay)?;tokio_pause().await;
                    assert_eq!(capture::status(&handle).region,Some(expected));assert!(main.is_visible().map_err(|e|e.to_string())?);Ok(())
                }.await;
                match result{Ok(())=>{println!("Native capture smoke passed: preview, selection, region capture, hide/restore, shortcut registration/replacement/disable, cancellation preserves region.");handle.exit(0);},Err(error)=>{eprintln!("Native capture smoke failed: {error}");handle.exit(1);}}
            });Ok(())
        }).run(tauri::generate_context!()).expect("capture smoke app");
}
async fn tokio_pause() {
    tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(std::time::Duration::from_millis(300))
    })
    .await
    .unwrap();
}
