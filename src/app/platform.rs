//! Platform-specific initialization and workarounds.

/// On Windows, Alt+Space opens the system window menu (Restore/Move/Size/Close)
/// even for borderless windows. This happens because Windows processes the key
/// combo in the message loop *before* egui can see it.
///
/// We install a thread-level keyboard hook (`WH_KEYBOARD`) that intercepts
/// Alt+Space at the earliest point in the message pipeline and blocks it.
/// When blocked, we set an atomic flag so the app can toggle the command
/// palette from its `update()` method.
///
/// This approach is more reliable than WndProc subclassing because:
/// - It runs before `TranslateMessage`/`DispatchMessage`
/// - It doesn't depend on getting the correct HWND
/// - It isn't affected by winit resetting the WndProc
#[cfg(target_os = "windows")]
mod win32 {
    use std::sync::atomic::{AtomicBool, Ordering};

    type WPARAM = usize;
    type LPARAM = isize;
    type LRESULT = isize;

    const WH_KEYBOARD: i32 = 2;
    const HC_ACTION: i32 = 0;
    const VK_SPACE: usize = 0x20;
    const KF_ALTDOWN: u32 = 0x2000;

    extern "system" {
        fn SetWindowsHookExW(
            id_hook: i32,
            lpfn: unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT,
            hmod: isize,
            thread_id: u32,
        ) -> isize;
        fn CallNextHookEx(hhk: isize, code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
        fn GetCurrentThreadId() -> u32;
    }

    /// Set by the keyboard hook when Alt+Space is intercepted.
    static PALETTE_TOGGLED: AtomicBool = AtomicBool::new(false);

    unsafe extern "system" fn keyboard_hook_proc(
        code: i32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if code == HC_ACTION {
            let vk = wparam;
            let flags = ((lparam as u32) >> 16) & 0xFFFF;
            let is_key_down = (lparam >> 31) & 1 == 0;

            if vk == VK_SPACE && (flags & KF_ALTDOWN) != 0 && is_key_down {
                PALETTE_TOGGLED.store(true, Ordering::Release);
                return 1; // Block this message — prevents WM_SYSCOMMAND / system menu
            }
        }
        unsafe { CallNextHookEx(0, code, wparam, lparam) }
    }

    pub(super) fn install_keyboard_hook() {
        unsafe {
            let tid = GetCurrentThreadId();
            let hook = SetWindowsHookExW(WH_KEYBOARD, keyboard_hook_proc, 0, tid);
            if hook == 0 {
                log::warn!("Failed to install Alt+Space keyboard hook (tid={})", tid);
            } else {
                log::info!("Alt+Space keyboard hook installed (tid={})", tid);
            }
        }
    }

    pub(super) fn take_palette_toggle() -> bool {
        PALETTE_TOGGLED.swap(false, Ordering::AcqRel)
    }
}

/// Atomically un-maximize the active window so it restores *under the cursor*
/// (Windows only).
///
/// Used when the user drags the custom title bar of a maximized window.
/// Restoring via `ViewportCommand::Maximized(false)` + `OuterPosition` is not
/// atomic: the OS restore transition targets the window's saved restore rect,
/// so the window visibly lands at its pre-maximize position for a moment
/// before the reposition command moves it to the cursor.
///
/// `SetWindowPlacement` avoids that by rewriting the saved restore rect and
/// issuing the restore in a single call — the window restores directly under
/// the cursor, matching native caption drag-out behaviour.
///
/// The cursor keeps its proportional x-position over the title bar, and the
/// window top stays at the top of the work area (like a native drag-out).
///
/// Returns `false` if any Win32 call fails (or on non-Windows platforms);
/// the caller should fall back to viewport commands.
#[cfg(target_os = "windows")]
pub(crate) fn begin_maximized_drag_out() -> bool {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetCursorPos, GetWindowPlacement, GetWindowRect, SetWindowPlacement, SW_RESTORE,
        WINDOWPLACEMENT, WINDOWPLACEMENT_FLAGS,
    };

    unsafe {
        let hwnd = GetActiveWindow();
        if hwnd.0.is_null() {
            return false;
        }

        let mut wp = WINDOWPLACEMENT {
            length: std::mem::size_of::<WINDOWPLACEMENT>() as u32,
            ..Default::default()
        };
        if GetWindowPlacement(hwnd, &mut wp).is_err() {
            return false;
        }

        let mut wnd_rect = RECT::default();
        if GetWindowRect(hwnd, &mut wnd_rect).is_err() {
            return false;
        }

        let mut cursor = POINT::default();
        if GetCursorPos(&mut cursor).is_err() {
            return false;
        }

        let restored_w = wp.rcNormalPosition.right - wp.rcNormalPosition.left;
        let restored_h = wp.rcNormalPosition.bottom - wp.rcNormalPosition.top;
        if restored_w <= 0 || restored_h <= 0 {
            return false;
        }

        // Cursor's proportional x across the maximized window, applied to the
        // restored width so the grab point stays over the title bar.
        let max_w = (wnd_rect.right - wnd_rect.left).max(1);
        let frac_x = ((cursor.x - wnd_rect.left) as f64 / max_w as f64).clamp(0.0, 1.0);
        let target_x = cursor.x - (frac_x * restored_w as f64).round() as i32;

        // rcNormalPosition uses *workspace* coordinates (screen coordinates
        // minus the taskbar inset of the containing monitor).
        let hmon = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let (off_x, off_y, work_top) = if GetMonitorInfoW(hmon, &mut mi).as_bool() {
            (
                mi.rcWork.left - mi.rcMonitor.left,
                mi.rcWork.top - mi.rcMonitor.top,
                mi.rcWork.top,
            )
        } else {
            (0, 0, wnd_rect.top)
        };

        // Keep the window top at the work-area top: a maximized borderless
        // window overhangs the monitor by a few px (phantom resize border),
        // which must not push the restored window off-screen.
        let target_y = wnd_rect.top.max(work_top);

        wp.rcNormalPosition = RECT {
            left: target_x - off_x,
            top: target_y - off_y,
            right: target_x - off_x + restored_w,
            bottom: target_y - off_y + restored_h,
        };
        wp.flags = WINDOWPLACEMENT_FLAGS(0);
        wp.showCmd = SW_RESTORE.0 as u32;

        SetWindowPlacement(hwnd, &wp).is_ok()
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn begin_maximized_drag_out() -> bool {
    false
}

/// Install the platform keyboard hook (call once at startup).
pub(crate) fn install_platform_hooks() {
    #[cfg(target_os = "windows")]
    win32::install_keyboard_hook();
}

/// Returns `true` (and clears the flag) if the platform hook intercepted
/// the palette toggle shortcut this frame.
pub(crate) fn take_palette_toggle_from_hook() -> bool {
    #[cfg(target_os = "windows")]
    {
        return win32::take_palette_toggle();
    }
    #[cfg(not(target_os = "windows"))]
    false
}
