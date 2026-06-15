//! Platform hooks so markdown scroll continues while the cursor is over a video WebView.
//!
//! WebView2 consumes wheel input before winit/egui. A low-level mouse hook detects when the
//! cursor is over a known embed HWND, forwards `WM_MOUSEWHEEL` to Ferrite's parent window via
//! `PostMessageW` (so winit delivers it to egui), and swallows the event. HWND subclassing on
//! the embed tree is a secondary path when the hook does not run first.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::sync::Once;

#[cfg(windows)]
use eframe::egui::{self, Modifiers, TouchPhase, Vec2};

#[cfg(windows)]
thread_local! {
    static SUBCLASSED_HWNDS: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
    static WEBVIEW_CONTAINER_HWNDS: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
    static MAIN_WINDOW_HWND: Cell<isize> = const { Cell::new(0) };
    static PENDING_WHEEL: RefCell<Vec<PendingWheel>> = RefCell::new(Vec::new());
}

#[cfg(windows)]
static WHEEL_HOOK_ONCE: Once = Once::new();

#[cfg(windows)]
struct PendingWheel {
    delta: Vec2,
    modifiers: Modifiers,
}

#[cfg(windows)]
const WHEEL_DELTA: f32 = 120.0;

#[cfg(windows)]
const WHEEL_FORWARD_SUBCLASS_ID: usize = 0xFE_B177_01;

#[cfg(windows)]
const MK_CONTROL: u16 = 0x0008;

#[cfg(windows)]
const MK_SHIFT: u16 = 0x0004;

/// Record Ferrite's parent HWND for wheel forwarding (call each rendered frame).
#[cfg(windows)]
pub fn set_main_window_from_parent(parent: &super::video_render::VideoWebViewParent) {
    if let Some(hwnd) = parent.win32_hwnd() {
        MAIN_WINDOW_HWND.with(|h| h.set(hwnd));
    }
}

#[cfg(not(windows))]
pub fn set_main_window_from_parent(_parent: &super::video_render::VideoWebViewParent) {}

#[cfg(windows)]
pub fn ensure_low_level_wheel_hook() {
    WHEEL_HOOK_ONCE.call_once(|| unsafe {
        use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
        use windows::Win32::UI::WindowsAndMessaging::{
            CallNextHookEx, SetWindowsHookExW, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_MOUSEHWHEEL,
            WM_MOUSEWHEEL,
        };

        unsafe extern "system" fn low_level_mouse_proc(
            code: i32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            if code >= 0 {
                let msg = wparam.0 as u32;
                if msg == WM_MOUSEWHEEL || msg == WM_MOUSEHWHEEL {
                    let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
                    if pointer_over_video_webview(info.pt) {
                        if !post_wheel_to_main_window(msg, info) {
                            let hi = ((info.mouseData >> 16) & 0xFFFF) as i16 as f32;
                            let lines = hi / WHEEL_DELTA;
                            let keys = current_wheel_key_state();
                            if msg == WM_MOUSEWHEEL {
                                queue_wheel(Vec2::new(0.0, lines), keys);
                            } else {
                                queue_wheel(Vec2::new(-lines, 0.0), keys);
                            }
                        }
                        return LRESULT(1);
                    }
                }
            }
            CallNextHookEx(None, code, wparam, lparam)
        }

        let _ = SetWindowsHookExW(WH_MOUSE_LL, Some(low_level_mouse_proc), None, 0);
    });
}

#[cfg(not(windows))]
pub fn ensure_low_level_wheel_hook() {}

/// Inject queued wheel events (fallback when `PostMessageW` is unavailable).
#[cfg(windows)]
pub fn drain_pending_wheel_into_egui(ctx: &egui::Context) {
    let pending: Vec<PendingWheel> = PENDING_WHEEL.with(|q| q.borrow_mut().drain(..).collect());
    if pending.is_empty() {
        return;
    }

    ctx.input_mut(|input| {
        for wheel in pending {
            if wheel.delta.length_sq() < 1.0e-8 {
                continue;
            }
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: wheel.delta,
                modifiers: wheel.modifiers,
                phase: TouchPhase::Move,
            });
        }
    });
    ctx.request_repaint();
}

#[cfg(not(windows))]
pub fn drain_pending_wheel_into_egui(_ctx: &eframe::egui::Context) {}

#[cfg(windows)]
fn main_window_hwnd() -> windows::Win32::Foundation::HWND {
    use windows::Win32::Foundation::HWND;
    let raw = MAIN_WINDOW_HWND.with(|h| h.get());
    HWND(raw as *mut _)
}

#[cfg(windows)]
fn pointer_over_video_webview(pt: windows::Win32::Foundation::POINT) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetParent, WindowFromPoint};

    unsafe {
        let mut hwnd = WindowFromPoint(pt);
        if hwnd.0.is_null() {
            return false;
        }
        WEBVIEW_CONTAINER_HWNDS.with(|containers| {
            let containers = containers.borrow();
            while !hwnd.0.is_null() {
                if containers.contains(&(hwnd.0 as isize)) {
                    return true;
                }
                hwnd = GetParent(hwnd).unwrap_or(HWND::default());
            }
            false
        })
    }
}

#[cfg(windows)]
fn post_wheel_to_main_window(
    msg: u32,
    info: &windows::Win32::UI::WindowsAndMessaging::MSLLHOOKSTRUCT,
) -> bool {
    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

    let main = main_window_hwnd();
    if main.0.is_null() {
        return false;
    }

    unsafe {
        let keys = current_wheel_key_state() as u32;
        let delta = ((info.mouseData >> 16) & 0xFFFF) as u32;
        let wheel_wparam = WPARAM((delta as usize) << 16 | keys as usize);
        let lparam = LPARAM(
            ((info.pt.x as u32) & 0xFFFF) as isize
                | (((info.pt.y as u32) & 0xFFFF) as isize) << 16,
        );
        PostMessageW(Some(main), msg, wheel_wparam, lparam).is_ok()
    }
}

#[cfg(windows)]
fn post_wheel_message(
    msg: u32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

    let main = main_window_hwnd();
    if main.0.is_null() {
        return false;
    }
    unsafe { PostMessageW(Some(main), msg, wparam, lparam).is_ok() }
}

#[cfg(windows)]
fn current_wheel_key_state() -> u16 {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_SHIFT,
    };

    let mut keys = 0u16;
    unsafe {
        if GetAsyncKeyState(VK_CONTROL.0 as i32) < 0 {
            keys |= MK_CONTROL;
        }
        if GetAsyncKeyState(VK_SHIFT.0 as i32) < 0 {
            keys |= MK_SHIFT;
        }
    }
    keys
}

#[cfg(windows)]
fn queue_wheel(delta: Vec2, key_state: u16) {
    let mut modifiers = Modifiers::default();
    if key_state & MK_CONTROL != 0 {
        modifiers.ctrl = true;
    }
    if key_state & MK_SHIFT != 0 {
        modifiers.shift = true;
    }
    PENDING_WHEEL.with(|q| {
        q.borrow_mut().push(PendingWheel { delta, modifiers });
    });
}

#[cfg(windows)]
fn wheel_delta_from_wparam(wparam: windows::Win32::Foundation::WPARAM) -> (u16, f32) {
    let key_state = (wparam.0 & 0xFFFF) as u16;
    let hi = ((wparam.0 >> 16) & 0xFFFF) as i16 as f32;
    (key_state, hi / WHEEL_DELTA)
}

#[cfg(windows)]
pub fn install_wheel_forwarding(webview: &wry::WebView) {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::Shell::SetWindowSubclass;
    use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetParent};
    use wry::WebViewExtWindows;

    unsafe extern "system" fn webview_wheel_subclass_proc(
        hwnd: windows::Win32::Foundation::HWND,
        msg: u32,
        wparam: windows::Win32::Foundation::WPARAM,
        lparam: windows::Win32::Foundation::LPARAM,
        _subclass_id: usize,
        _ref_data: usize,
    ) -> windows::Win32::Foundation::LRESULT {
        use windows::Win32::Foundation::LRESULT;
        use windows::Win32::UI::Shell::DefSubclassProc;
        use windows::Win32::UI::WindowsAndMessaging::{
            WM_MOUSEHWHEEL, WM_MOUSEWHEEL, WM_POINTERHWHEEL, WM_POINTERWHEEL,
        };

        match msg {
            WM_MOUSEWHEEL | WM_POINTERWHEEL | WM_MOUSEHWHEEL | WM_POINTERHWHEEL => {
                if post_wheel_message(msg, wparam, lparam) {
                    return LRESULT(0);
                }
                let (keys, lines) = wheel_delta_from_wparam(wparam);
                if msg == WM_MOUSEWHEEL || msg == WM_POINTERWHEEL {
                    queue_wheel(Vec2::new(0.0, lines), keys);
                } else {
                    queue_wheel(Vec2::new(-lines, 0.0), keys);
                }
                return LRESULT(0);
            }
            _ => DefSubclassProc(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn subclass_one(hwnd: HWND) {
        let key = hwnd.0 as isize;
        if SUBCLASSED_HWNDS.with(|s| s.borrow().contains(&key)) {
            return;
        }
        if SetWindowSubclass(
            hwnd,
            Some(webview_wheel_subclass_proc),
            WHEEL_FORWARD_SUBCLASS_ID,
            0,
        )
        .as_bool()
        {
            SUBCLASSED_HWNDS.with(|s| {
                s.borrow_mut().insert(key);
            });
        }
    }

    unsafe extern "system" fn enum_subclass_child(hwnd: HWND, _lparam: LPARAM) -> BOOL {
        subclass_hwnd_tree(hwnd);
        BOOL::from(true)
    }

    unsafe fn subclass_hwnd_tree(hwnd: HWND) {
        subclass_one(hwnd);
        let _ = EnumChildWindows(Some(hwnd), Some(enum_subclass_child), LPARAM(0));
    }

    let controller = webview.controller();
    let mut container = HWND::default();
    if unsafe { controller.ParentWindow(&mut container) }.is_err() || container.0.is_null() {
        return;
    }

    unsafe {
        let parent = GetParent(container).unwrap_or(HWND::default());
        if !parent.0.is_null() {
            MAIN_WINDOW_HWND.with(|h| h.set(parent.0 as isize));
        }
        WEBVIEW_CONTAINER_HWNDS.with(|s| {
            s.borrow_mut().insert(container.0 as isize);
        });
        subclass_hwnd_tree(container);
    }
}

#[cfg(not(windows))]
pub fn install_wheel_forwarding(_webview: &wry::WebView) {}
