//! Explicit, on-demand screen-region capture. Nothing is recorded to disk.
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use std::{
    io::Cursor,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub const DEFAULT_HOTKEY: &str = "y";
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
#[derive(Deserialize)]
pub struct Selection {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub region: Option<ScreenRect>,
    pub hotkey: Option<String>,
    pub configured_hotkey: String,
    pub hotkey_enabled: bool,
    pub shortcut_error: Option<String>,
    pub selecting: bool,
}
#[derive(Clone, Serialize)]
pub struct Captured {
    pub region: ScreenRect,
    pub image: String,
}
#[derive(Serialize)]
pub struct Preview {
    pub bounds: ScreenRect,
    pub image: String,
}
struct Frame {
    bounds: ScreenRect,
    pixels: RgbaImage,
}
#[derive(Default)]
struct Inner {
    region: Option<ScreenRect>,
    preview: Option<Frame>,
    restore: Option<native::HiddenMain>,
    hotkey: Option<String>,
    configured_hotkey: Option<String>,
    shortcut_error: Option<String>,
}
#[derive(Default)]
pub struct CaptureState {
    inner: Mutex<Inner>,
    hotkey_update: Mutex<()>,
    busy: AtomicBool,
}

fn png(pixels: RgbaImage) -> Result<String, String> {
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(pixels)
        .write_to(&mut out, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(out.into_inner())
    ))
}
fn main_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "main" {
        return Err("This action belongs to the assistant window.".into());
    }
    Ok(())
}
fn overlay_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() != "capture-overlay" {
        return Err("This action belongs to the capture selector.".into());
    }
    Ok(())
}
pub fn status(app: &AppHandle) -> CaptureStatus {
    let state = app.state::<CaptureState>();
    let inner = state.inner.lock().unwrap_or_else(|e| e.into_inner());
    CaptureStatus {
        region: inner.region,
        hotkey: inner.hotkey.clone(),
        configured_hotkey: inner
            .configured_hotkey
            .clone()
            .unwrap_or_else(|| DEFAULT_HOTKEY.into()),
        hotkey_enabled: inner.hotkey.is_some(),
        shortcut_error: inner.shortcut_error.clone(),
        selecting: inner.preview.is_some(),
    }
}

pub fn selected_rect(bounds: ScreenRect, selection: &Selection) -> Result<ScreenRect, String> {
    if ![selection.x, selection.y, selection.width, selection.height]
        .iter()
        .all(|n| n.is_finite())
        || selection.x < 0.0
        || selection.y < 0.0
        || selection.width <= 0.0
        || selection.height <= 0.0
        || selection.x + selection.width > 1.000001
        || selection.y + selection.height > 1.000001
    {
        return Err("Select a rectangle inside the screen.".into());
    }
    let x = (selection.x * bounds.width as f64).floor() as u32;
    let y = (selection.y * bounds.height as f64).floor() as u32;
    let right = ((selection.x + selection.width) * bounds.width as f64)
        .ceil()
        .min(bounds.width as f64) as u32;
    let bottom = ((selection.y + selection.height) * bounds.height as f64)
        .ceil()
        .min(bounds.height as f64) as u32;
    if right.saturating_sub(x) < 32 || bottom.saturating_sub(y) < 32 {
        return Err(
            "Select a larger area around the board and counter (at least 32 × 32 pixels).".into(),
        );
    }
    Ok(ScreenRect {
        x: bounds.x + x as i32,
        y: bounds.y + y as i32,
        width: right - x,
        height: bottom - y,
    })
}
fn contained(rect: ScreenRect, bounds: ScreenRect) -> bool {
    rect.width >= 32
        && rect.height >= 32
        && rect.x >= bounds.x
        && rect.y >= bounds.y
        && rect.x as i64 + rect.width as i64 <= bounds.x as i64 + bounds.width as i64
        && rect.y as i64 + rect.height as i64 <= bounds.y as i64 + bounds.height as i64
}

#[tauri::command]
pub fn capture_status(app: AppHandle, window: WebviewWindow) -> Result<CaptureStatus, String> {
    main_only(&window)?;
    Ok(status(&app))
}

fn restore_selection(app: &AppHandle) -> bool {
    let state = app.state::<CaptureState>();
    let (restore, had_selection) = {
        let mut inner = state.inner.lock().unwrap_or_else(|e| e.into_inner());
        let had = inner.preview.take().is_some();
        (inner.restore.take(), had)
    };
    drop(restore);
    if had_selection {
        state.busy.store(false, Ordering::Release);
    }
    had_selection
}

#[tauri::command]
pub async fn select_capture_area(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    main_only(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<CaptureState>();
        if state
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            return Err("A capture is already in progress.".into());
        }
        let result = (|| {
            let hidden = native::hide_main(&app)?;
            std::thread::sleep(std::time::Duration::from_millis(220));
            let bounds = native::desktop_bounds();
            let pixels = native::capture(bounds)?;
            {
                let mut inner = state.inner.lock().unwrap_or_else(|e| e.into_inner());
                inner.preview = Some(Frame { bounds, pixels });
                inner.restore = Some(hidden);
            }
            let overlay = WebviewWindowBuilder::new(
                &app,
                "capture-overlay",
                WebviewUrl::App("capture".into()),
            )
            .title("Select Minesweeper capture area")
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .visible(false)
            .build()
            .map_err(|e| e.to_string())?;
            let handle = app.clone();
            overlay.on_window_event(move |event| {
                if matches!(event, tauri::WindowEvent::Destroyed) && restore_selection(&handle) {
                    let _ = handle.emit_to("main", "capture-selection-cancelled", ());
                }
            });
            overlay
                .set_position(PhysicalPosition::new(bounds.x, bounds.y))
                .map_err(|e| e.to_string())?;
            overlay
                .set_size(PhysicalSize::new(bounds.width, bounds.height))
                .map_err(|e| e.to_string())?;
            overlay.show().map_err(|e| e.to_string())?;
            overlay.set_focus().map_err(|e| e.to_string())?;
            Ok(())
        })();
        if result.is_err() {
            restore_selection(&app);
            state.busy.store(false, Ordering::Release);
            if let Some(overlay) = app.get_webview_window("capture-overlay") {
                let _ = overlay.close();
            }
        }
        result
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn capture_preview(app: AppHandle, window: WebviewWindow) -> Result<Preview, String> {
    overlay_only(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<CaptureState>();
        let (bounds, pixels) = {
            let inner = state.inner.lock().unwrap_or_else(|e| e.into_inner());
            let frame = inner
                .preview
                .as_ref()
                .ok_or("No area selection is active.")?;
            (frame.bounds, frame.pixels.clone())
        };
        Ok(Preview {
            bounds,
            image: png(if pixels.width() > 3840 || pixels.height() > 2160 {
                image::imageops::thumbnail(&pixels, 3840, 2160)
            } else {
                pixels
            })?,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn finish_capture_selection(
    app: AppHandle,
    window: WebviewWindow,
    selection: Selection,
) -> Result<(), String> {
    overlay_only(&window)?;
    let state = app.state::<CaptureState>();
    let captured = {
        let mut inner = state.inner.lock().unwrap_or_else(|e| e.into_inner());
        let frame = inner
            .preview
            .as_ref()
            .ok_or("No area selection is active.")?;
        let rect = selected_rect(frame.bounds, &selection)?;
        let pixels = image::imageops::crop_imm(
            &frame.pixels,
            (rect.x - frame.bounds.x) as u32,
            (rect.y - frame.bounds.y) as u32,
            rect.width,
            rect.height,
        )
        .to_image();
        let image = png(pixels)?;
        inner.region = Some(rect);
        Captured {
            region: rect,
            image,
        }
    };
    restore_selection(&app);
    window.close().map_err(|e| e.to_string())?;
    app.emit_to("main", "capture-region-selected", captured)
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub fn cancel_capture_selection(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    overlay_only(&window)?;
    restore_selection(&app);
    window.close().map_err(|e| e.to_string())?;
    app.emit_to("main", "capture-selection-cancelled", ())
        .map_err(|e| e.to_string())
}

struct BusyGuard<'a>(&'a AtomicBool);
impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}
#[tauri::command]
pub async fn capture_region(app: AppHandle, window: WebviewWindow) -> Result<Captured, String> {
    main_only(&window)?;
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<CaptureState>();
        if state
            .busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
            .is_err()
        {
            return Err("A capture is already in progress.".into());
        }
        let _busy = BusyGuard(&state.busy);
        let region = state
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .region
            .ok_or("Choose a capture area first.")?;
        if !contained(region, native::desktop_bounds()) {
            return Err("The display layout changed. Select the capture area again.".into());
        }
        let hidden = native::hide_main(&app)?;
        std::thread::sleep(std::time::Duration::from_millis(180));
        let pixels = native::capture(region);
        drop(hidden);
        Ok(Captured {
            region,
            image: png(pixels?)?,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn configure_hotkey(app: &AppHandle, hotkey: Option<String>) -> Result<CaptureStatus, String> {
    let enabled = hotkey.is_some();
    update_hotkey(app, hotkey, Some(enabled))
}
pub fn set_hotkey_enabled(app: &AppHandle, enabled: bool) -> Result<CaptureStatus, String> {
    update_hotkey(app, None, Some(enabled))
}
pub fn change_hotkey_binding(app: &AppHandle, hotkey: String) -> Result<CaptureStatus, String> {
    update_hotkey(app, Some(hotkey), None)
}
fn update_hotkey(
    app: &AppHandle,
    binding: Option<String>,
    enabled: Option<bool>,
) -> Result<CaptureStatus, String> {
    let state = app.state::<CaptureState>();
    let _update = state
        .hotkey_update
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (old, remembered) = {
        let inner = state.inner.lock().unwrap_or_else(|e| e.into_inner());
        (
            inner.hotkey.clone(),
            inner
                .configured_hotkey
                .clone()
                .unwrap_or_else(|| DEFAULT_HOTKEY.into()),
        )
    };
    let key = binding.unwrap_or(remembered);
    validate_hotkey(&key)?;
    let shortcut = native_shortcut(&key).map_err(|e| format!("Invalid capture shortcut: {e}"))?;
    let hotkey = if enabled.unwrap_or(old.is_some()) {
        Some(key.clone())
    } else {
        None
    };
    let parsed = hotkey.as_ref().map(|_| shortcut);
    if old == hotkey {
        let mut inner = state.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.configured_hotkey = Some(key);
        inner.shortcut_error = None;
        drop(inner);
        return Ok(status(app));
    }
    if let Some(shortcut) = parsed {
        app.global_shortcut().register(shortcut).map_err(|e| {
            format!("That capture shortcut is unavailable. Choose another combination. {e}")
        })?;
    }
    if let Some(old) = &old {
        if let Err(error) = app
            .global_shortcut()
            .unregister(native_shortcut(old).map_err(|e| e.to_string())?)
        {
            if let Some(shortcut) = parsed {
                let _ = app.global_shortcut().unregister(shortcut);
            }
            return Err(error.to_string());
        }
    }
    let mut inner = state.inner.lock().unwrap_or_else(|e| e.into_inner());
    inner.hotkey = hotkey;
    inner.configured_hotkey = Some(key);
    inner.shortcut_error = None;
    drop(inner);
    Ok(status(app))
}
fn native_shortcut(value: &str) -> Result<Shortcut, tauri_plugin_global_shortcut::Error> {
    let normalized = value
        .split('+')
        .map(|part| {
            if part.eq_ignore_ascii_case("meta") || part.eq_ignore_ascii_case("win") {
                "Super"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join("+");
    normalized.parse::<Shortcut>().map_err(Into::into)
}
pub fn validate_hotkey(value: &str) -> Result<(), String> {
    let lower = value.to_lowercase();
    let parts: Vec<_> = lower.split('+').collect();
    let key = parts.last().copied().unwrap_or("");
    let modified = parts.iter().any(|p| {
        matches!(
            *p,
            "ctrl" | "control" | "alt" | "meta" | "super" | "win" | "command"
        )
    });
    let function = key
        .strip_prefix('f')
        .and_then(|s| s.parse::<u8>().ok())
        .is_some_and(|n| (1..=24).contains(&n));
    let single = key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric();
    if value.len() > 80
        || (!modified && !function && !single)
        || matches!(key, "escape" | "tab")
        || (parts
            .iter()
            .any(|p| matches!(*p, "ctrl" | "control" | "meta" | "command"))
            && matches!(key, "c" | "v" | "x" | "a" | "w" | "r" | "t" | "q"))
        || (parts.contains(&"alt") && key == "f4")
    {
        return Err("Choose a letter, digit, function key, or key combination. Clipboard and system shortcuts are reserved.".into());
    }
    Ok(())
}
#[tauri::command]
pub async fn set_capture_hotkey(
    app: AppHandle,
    window: WebviewWindow,
    hotkey: Option<String>,
) -> Result<CaptureStatus, String> {
    main_only(&window)?;
    match hotkey {
        Some(key) => change_hotkey_binding(&app, key),
        None => set_hotkey_enabled(&app, false),
    }
}
#[tauri::command]
pub async fn set_capture_hotkey_enabled(
    app: AppHandle,
    window: WebviewWindow,
    enabled: bool,
) -> Result<CaptureStatus, String> {
    main_only(&window)?;
    set_hotkey_enabled(&app, enabled)
}
pub fn init(app: &AppHandle) {
    // Remember Y, but do not intercept typing until the user explicitly enables it.
    if let Err(error) = set_hotkey_enabled(app, false) {
        app.state::<CaptureState>()
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .shortcut_error = Some(error);
    }
}

#[cfg(windows)]
mod native {
    use super::*;
    use std::ptr::null_mut;
    use windows_sys::Win32::{
        Graphics::Gdi::*,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    };
    struct DpiGuard(DPI_AWARENESS_CONTEXT);
    impl DpiGuard {
        fn new() -> Self {
            Self(unsafe {
                SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
            })
        }
    }
    impl Drop for DpiGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    SetThreadDpiAwarenessContext(self.0);
                }
            }
        }
    }
    pub struct HiddenMain {
        handle: usize,
        visible: bool,
        foreground: bool,
    }
    impl Drop for HiddenMain {
        fn drop(&mut self) {
            if self.visible {
                unsafe {
                    // Preserve the previous Z order as well as keyboard focus.
                    // A hotkey must not raise a background assistant over the game.
                    SetWindowPos(
                        self.handle as _,
                        std::ptr::null_mut(),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_SHOWWINDOW,
                    );
                    if self.foreground {
                        SetForegroundWindow(self.handle as _);
                    }
                }
            }
        }
    }
    pub fn hide_main(app: &AppHandle) -> Result<HiddenMain, String> {
        let main = app
            .get_webview_window("main")
            .ok_or("Assistant window is unavailable.")?;
        let hwnd = main.hwnd().map_err(|e| e.to_string())?.0;
        unsafe {
            let hidden = HiddenMain {
                handle: hwnd as usize,
                visible: IsWindowVisible(hwnd) != 0,
                foreground: GetForegroundWindow() == hwnd,
            };
            if hidden.visible {
                ShowWindow(hwnd, SW_HIDE);
            }
            Ok(hidden)
        }
    }
    pub fn desktop_bounds() -> ScreenRect {
        let _dpi = DpiGuard::new();
        unsafe {
            ScreenRect {
                x: GetSystemMetrics(SM_XVIRTUALSCREEN),
                y: GetSystemMetrics(SM_YVIRTUALSCREEN),
                width: GetSystemMetrics(SM_CXVIRTUALSCREEN).max(0) as u32,
                height: GetSystemMetrics(SM_CYVIRTUALSCREEN).max(0) as u32,
            }
        }
    }
    struct Gdi {
        screen: HDC,
        memory: HDC,
        bitmap: HBITMAP,
        old: HGDIOBJ,
    }
    impl Drop for Gdi {
        fn drop(&mut self) {
            unsafe {
                if !self.memory.is_null() && !self.old.is_null() {
                    SelectObject(self.memory, self.old);
                }
                if !self.bitmap.is_null() {
                    DeleteObject(self.bitmap);
                }
                if !self.memory.is_null() {
                    DeleteDC(self.memory);
                }
                if !self.screen.is_null() {
                    ReleaseDC(null_mut(), self.screen);
                }
            }
        }
    }
    pub fn capture(rect: ScreenRect) -> Result<RgbaImage, String> {
        let _dpi = DpiGuard::new();
        if rect.width == 0
            || rect.height == 0
            || rect.width as u64 * rect.height as u64 > 32_000_000
        {
            return Err(
                "This capture area is too large or unavailable. Use a smaller display region."
                    .into(),
            );
        }
        unsafe {
            let mut handles = Gdi {
                screen: GetDC(null_mut()),
                memory: null_mut(),
                bitmap: null_mut(),
                old: null_mut(),
            };
            if handles.screen.is_null() {
                return Err("Windows could not access the display.".into());
            }
            handles.memory = CreateCompatibleDC(handles.screen);
            handles.bitmap =
                CreateCompatibleBitmap(handles.screen, rect.width as i32, rect.height as i32);
            if handles.memory.is_null() || handles.bitmap.is_null() {
                return Err("Windows could not allocate the capture image.".into());
            }
            handles.old = SelectObject(handles.memory, handles.bitmap);
            // Copy display pixels only. We never draw GetCursorInfo/DrawIconEx,
            // so the system pointer is not composited into the screenshot.
            if BitBlt(
                handles.memory,
                0,
                0,
                rect.width as i32,
                rect.height as i32,
                handles.screen,
                rect.x,
                rect.y,
                SRCCOPY | CAPTUREBLT,
            ) == 0
            {
                return Err("Windows could not capture this area.".into());
            }
            SelectObject(handles.memory, handles.old);
            handles.old = null_mut();
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = rect.width as i32;
            info.bmiHeader.biHeight = -(rect.height as i32);
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;
            let mut data = vec![0u8; rect.width as usize * rect.height as usize * 4];
            if GetDIBits(
                handles.screen,
                handles.bitmap,
                0,
                rect.height,
                data.as_mut_ptr() as _,
                &mut info,
                DIB_RGB_COLORS,
            ) != rect.height as i32
            {
                return Err("Windows could not read the captured pixels.".into());
            }
            for pixel in data.chunks_exact_mut(4) {
                pixel.swap(0, 2);
                pixel[3] = 255;
            }
            RgbaImage::from_raw(rect.width, rect.height, data)
                .ok_or("Invalid capture image.".into())
        }
    }
}
#[cfg(not(windows))]
mod native {
    use super::*;
    pub struct HiddenMain;
    pub fn hide_main(_: &AppHandle) -> Result<HiddenMain, String> {
        Err("Screen-region capture is currently available on Windows.".into())
    }
    pub fn desktop_bounds() -> ScreenRect {
        ScreenRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
    pub fn capture(_: ScreenRect) -> Result<RgbaImage, String> {
        Err("Screen-region capture is currently available on Windows.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selects_physical_pixels_on_negative_origin_monitors() {
        let bounds = ScreenRect {
            x: -1920,
            y: -200,
            width: 3840,
            height: 2160,
        };
        let rect = selected_rect(
            bounds,
            &Selection {
                x: 0.25,
                y: 0.25,
                width: 0.5,
                height: 0.5,
            },
        )
        .unwrap();
        assert_eq!(
            rect,
            ScreenRect {
                x: -960,
                y: 340,
                width: 1920,
                height: 1080
            }
        );
        assert!(contained(rect, bounds));
    }
    #[test]
    fn rejects_invalid_and_tiny_selections() {
        let bounds = ScreenRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        for s in [
            Selection {
                x: -0.1,
                y: 0.0,
                width: 0.5,
                height: 0.5,
            },
            Selection {
                x: 0.8,
                y: 0.0,
                width: 0.5,
                height: 0.5,
            },
            Selection {
                x: 0.0,
                y: 0.0,
                width: 0.001,
                height: 0.001,
            },
            Selection {
                x: f64::NAN,
                y: 0.0,
                width: 0.5,
                height: 0.5,
            },
        ] {
            assert!(selected_rect(bounds, &s).is_err());
        }
    }
    #[test]
    fn allows_single_keys_and_reserves_system_shortcuts() {
        for s in ["Space", "Ctrl+V", "Alt+F4", "Ctrl+Tab"] {
            assert!(validate_hotkey(s).is_err(), "{s}");
        }
        for s in [DEFAULT_HOTKEY, "q", "Shift+Q", "7", "Ctrl+Alt+F8", "F9"] {
            assert!(validate_hotkey(s).is_ok(), "{s}");
        }
        assert!(native_shortcut("Meta+F8").is_ok());
        assert!(native_shortcut("Ctrl+Alt+q").is_ok());
        assert!(native_shortcut(DEFAULT_HOTKEY).is_ok());
    }
}
