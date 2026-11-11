# Screen region capture and the blue target

## Workflow

1. Leave the actual game visible. Choose **Choose capture area** in the assistant's left sidebar.
2. On the frozen desktop preview, drag around the complete board and the remaining-mine counter. Reverse-direction drags work too. Press Enter or **Use this area**; Escape cancels.
3. The selected image scans immediately. Unambiguous scans are accepted automatically; uncertain scans keep the usual review workflow.
4. Use **Capture & analyze**, or enable **Global capture hotkey** in the left panel and press the default single key **Y**, to refresh and replan without copying or pasting. The key refreshes the existing rectangle and never changes the capture area.
5. Pause the hotkey before typing messages in another app. It starts paused on every launch. Pause/Disable unregisters the key from Windows and keeps the binding and area; Enable restores the saved key. Both the main panel and Settings offer this switch. Changing a binding while paused does not enable it. Conflicting/unavailable registrations are reported without replacing a working registration. The button continues to work when the global shortcut is disabled.

The region and shortcut remain in memory until the app closes. Existing board/session autosave still applies to the cropped source image and reconstructed board. Full-desktop selection previews are not written as image files or uploaded anywhere.

## Implementation

Rust owns the capture state, physical rectangle validation, bitmap capture, PNG encoding, global shortcut registration, and existing image recognition/solving. Svelte provides the selector, controls, and settings. The selector is a dedicated borderless Tauri window spanning the virtual desktop, including monitors with negative coordinates. Normalized pointer coordinates map back to the original physical pixels. The selection preview can be reduced for very large desktops; the actual selected image crops the full-resolution source frame.

The Windows implementation copies display pixels with GDI BitBlt / GetDIBits and never draws the cursor. The main assistant window hides during the snapshot and is restored without raising a background assistant or taking focus from the game. Shortcuts use Tauri's global-shortcut plugin. A busy guard prevents overlapping native captures, and frontend guards suppress duplicate triggers while capture/scanning is active. Board revisions protect against applying a capture after an intervening edit.

Only explicit button/hotkey requests capture anything. This is a visible screen-area capture, not a window-content capture: other applications covering the game can appear in the selected region. Keep the game visible and reselect after moving/resizing it. Game-rendered hover highlights and tooltips are not the system cursor and remain in the pixels. Display-layout changes that move the region outside the virtual screen require reselection. Captures over 32 million pixels are rejected. There is no continuous recording or automatic interaction with the game.

API references: [Microsoft BitBlt](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-bitblt), [Tauri global shortcuts](https://tauri.app/plugin/global-shortcut/).

## Blue recommendation

In probability mode, one primary recommendation receives a saturated blue background, cyan outline, and glow, with white percentage text. It remains distinct from the existing green/red probability extremes. Other cells may have the same lowest odds; blue does not turn a guess into a proven safe move. The toolbar crosshair and right-sidebar **Show blue target** scroll to it without changing zoom. Recalculation still preserves the user's view. In efficiency mode, the numbered action sequence remains the guide.

## Checks

40 Rust tests pass, including normalized selections with negative monitor origins, invalid/tiny rectangle rejection, and shortcut validation supporting single letters/digits. A Windows-native smoke application exercises real desktop bitmap acquisition, preview encoding, selection commit, region recapture, assistant hide/restore, real shortcut registration/replacement/disable/re-enable, and cancellation preserving the prior region. It also verifies a paused startup, remembered custom bindings, no registration while changing a paused binding, and unchanged capture area across toggles. No mouse or keyboard injection is used by this smoke test.

Browser integration checks the selection drag UI, reverse drags, Enter/Escape, invalid-selection recovery, automatic scan after selection, the capture button and hotkey event path, duplicate suppression, settings changes, zoom preservation, and a unique blue target. The native-event bridge uses fixture images, while recognition and solving run through the actual Rust engine. Existing layout, risk-style, settings, and zoom regression checks pass.

The Y-key regression also covers main-panel and Settings enable/disable/re-enable, keeping custom single-key bindings, typing normally while paused, ignoring stale hotkey events after pausing, same-area refresh without reopening the selector, and manual capture while paused.

To run the native smoke example on Windows, from `app/src-tauri`:

```powershell
cargo rustc --example capture_smoke -- -C link-arg=/MANIFEST:EMBED -C link-arg=/MANIFESTINPUT:examples/capture_smoke.exe.manifest
.\target\debug\examples\capture_smoke.exe
```

The example has a separate Common Controls/DPI manifest because the main executable's resource manifest is not automatically linked into Cargo examples.
