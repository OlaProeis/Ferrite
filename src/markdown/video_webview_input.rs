//! Platform hooks so markdown scroll continues while the cursor is over a video WebView.
//!
//! WebView2 consumes wheel input before winit/egui. A low-level mouse hook detects when the
//! cursor is over a known embed HWND, queues the wheel event (with the cursor's screen
//! position), and swallows it. The queue is drained into egui as synthetic
//! `PointerMoved` + `MouseWheel` events each frame.
//!
//! The cursor position must be injected alongside the wheel: while the cursor is over a
//! child WebView HWND, the main window receives `WM_MOUSELEAVE` and egui's pointer goes
//! `None` — a bare wheel event then has no scroll-area target and is silently dropped
//! (scroll "stops working" the moment the cursor enters a video).
//!
//! HWND subclassing on the embed tree is a secondary path when the hook does not run first.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

#[cfg(windows)]
use eframe::egui::{self, Modifiers, Pos2, TouchPhase, Vec2};

#[cfg(windows)]
thread_local! {
    static SUBCLASSED_HWNDS: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
    static WEBVIEW_CONTAINER_HWNDS: RefCell<HashSet<isize>> = RefCell::new(HashSet::new());
    /// Subclassed HWNDs grouped by their WebView container, so a destroyed
    /// container's entries can be purged (HWND values are reused by Windows).
    static CONTAINER_SUBCLASSED_HWNDS: RefCell<HashMap<isize, HashSet<isize>>> =
        RefCell::new(HashMap::new());
    static MAIN_WINDOW_HWND: Cell<isize> = const { Cell::new(0) };
    static PENDING_WHEEL: RefCell<Vec<PendingWheel>> = RefCell::new(Vec::new());
}

#[cfg(windows)]
static WHEEL_HOOK_ONCE: Once = Once::new();

/// When a video embed is in fullscreen mode, the wheel hook steps aside so the
/// player receives wheel input natively (and the document does not scroll).
static VIDEO_FULLSCREEN_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_video_fullscreen_active(active: bool) {
    VIDEO_FULLSCREEN_ACTIVE.store(active, Ordering::Relaxed);
}

pub fn video_fullscreen_active() -> bool {
    VIDEO_FULLSCREEN_ACTIVE.load(Ordering::Relaxed)
}

#[cfg(windows)]
struct PendingWheel {
    delta: Vec2,
    modifiers: Modifiers,
    /// Cursor position in physical screen coordinates when the wheel fired.
    screen_pt: (i32, i32),
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
                    // During video fullscreen the player owns the wheel (volume etc.)
                    // and the document must not scroll underneath.
                    if !video_fullscreen_active() && pointer_over_video_webview(info.pt) {
                        let hi = ((info.mouseData >> 16) & 0xFFFF) as i16 as f32;
                        let lines = hi / WHEEL_DELTA;
                        let keys = current_wheel_key_state();
                        let pt = (info.pt.x, info.pt.y);
                        if msg == WM_MOUSEWHEEL {
                            queue_wheel(Vec2::new(0.0, lines), keys, pt);
                        } else {
                            queue_wheel(Vec2::new(-lines, 0.0), keys, pt);
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

/// Inject queued wheel events into egui as synthetic `PointerMoved` + `MouseWheel`.
///
/// The pointer position is re-injected because the main window received
/// `WM_MOUSELEAVE` when the cursor moved over the WebView HWND; without a
/// position, egui has no scroll-area target and drops the wheel event.
#[cfg(windows)]
pub fn drain_pending_wheel_into_egui(ctx: &egui::Context) {
    let pending: Vec<PendingWheel> = PENDING_WHEEL.with(|q| q.borrow_mut().drain(..).collect());
    if pending.is_empty() {
        return;
    }

    let pixels_per_point = ctx.pixels_per_point();
    ctx.input_mut(|input| {
        for wheel in pending {
            if wheel.delta.length_sq() < 1.0e-8 {
                continue;
            }
            if let Some(pos) = screen_pt_to_egui_pos(wheel.screen_pt, pixels_per_point) {
                input.events.push(egui::Event::PointerMoved(pos));
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

/// Convert a physical screen point to egui (logical, client-relative) coordinates.
#[cfg(windows)]
fn screen_pt_to_egui_pos(screen_pt: (i32, i32), pixels_per_point: f32) -> Option<Pos2> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::ScreenToClient;

    let main = main_window_hwnd();
    if main.0.is_null() || pixels_per_point <= 0.0 {
        return None;
    }
    let mut pt = POINT {
        x: screen_pt.0,
        y: screen_pt.1,
    };
    if !unsafe { ScreenToClient(main, &mut pt) }.as_bool() {
        return None;
    }
    Some(Pos2::new(
        pt.x as f32 / pixels_per_point,
        pt.y as f32 / pixels_per_point,
    ))
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
fn current_wheel_key_state() -> u16 {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_SHIFT};

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
fn queue_wheel(delta: Vec2, key_state: u16, screen_pt: (i32, i32)) {
    let mut modifiers = Modifiers::default();
    if key_state & MK_CONTROL != 0 {
        modifiers.ctrl = true;
    }
    if key_state & MK_SHIFT != 0 {
        modifiers.shift = true;
    }
    PENDING_WHEEL.with(|q| {
        q.borrow_mut().push(PendingWheel {
            delta,
            modifiers,
            screen_pt,
        });
    });
    // Wake the event loop: nothing else is posted to the main window (the wheel
    // event is swallowed), so an idle egui would not repaint and drain the queue.
    super::video_render::request_video_repaint();
}

#[cfg(windows)]
fn wheel_delta_from_wparam(wparam: windows::Win32::Foundation::WPARAM) -> (u16, f32) {
    let key_state = (wparam.0 & 0xFFFF) as u16;
    let hi = ((wparam.0 >> 16) & 0xFFFF) as i16 as f32;
    (key_state, hi / WHEEL_DELTA)
}

/// Install wheel forwarding on the WebView's HWND tree.
///
/// Returns the container HWND (as `isize`) so the caller can unregister it via
/// [`unregister_webview_container`] when the WebView is destroyed. Safe to call
/// every frame: already-subclassed HWNDs are skipped, and WebView2 creates some
/// child HWNDs asynchronously after construction.
#[cfg(windows)]
pub fn install_wheel_forwarding(webview: &wry::WebView) -> Option<isize> {
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
                if video_fullscreen_active() {
                    // Let the fullscreen player handle wheel input natively.
                    return DefSubclassProc(hwnd, msg, wparam, lparam);
                }
                let (keys, lines) = wheel_delta_from_wparam(wparam);
                // Wheel messages carry the cursor position in *screen* coordinates
                // (signed 16-bit) in lparam.
                let pt = (
                    (lparam.0 & 0xFFFF) as u16 as i16 as i32,
                    ((lparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32,
                );
                if msg == WM_MOUSEWHEEL || msg == WM_POINTERWHEEL {
                    queue_wheel(Vec2::new(0.0, lines), keys, pt);
                } else {
                    queue_wheel(Vec2::new(-lines, 0.0), keys, pt);
                }
                LRESULT(0)
            }
            _ => DefSubclassProc(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn subclass_one(hwnd: HWND, container_key: isize) {
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
            CONTAINER_SUBCLASSED_HWNDS.with(|m| {
                m.borrow_mut().entry(container_key).or_default().insert(key);
            });
        }
    }

    unsafe extern "system" fn enum_subclass_child(hwnd: HWND, lparam: LPARAM) -> BOOL {
        subclass_hwnd_tree(hwnd, lparam.0);
        BOOL::from(true)
    }

    unsafe fn subclass_hwnd_tree(hwnd: HWND, container_key: isize) {
        subclass_one(hwnd, container_key);
        let _ = EnumChildWindows(Some(hwnd), Some(enum_subclass_child), LPARAM(container_key));
    }

    let controller = webview.controller();
    let mut container = HWND::default();
    if unsafe { controller.ParentWindow(&mut container) }.is_err() || container.0.is_null() {
        return None;
    }
    let container_key = container.0 as isize;

    unsafe {
        let parent = GetParent(container).unwrap_or(HWND::default());
        if !parent.0.is_null() {
            MAIN_WINDOW_HWND.with(|h| h.set(parent.0 as isize));
        }
        WEBVIEW_CONTAINER_HWNDS.with(|s| {
            s.borrow_mut().insert(container_key);
        });
        subclass_hwnd_tree(container, container_key);
    }
    Some(container_key)
}

#[cfg(not(windows))]
pub fn install_wheel_forwarding(_webview: &wry::WebView) -> Option<isize> {
    None
}

/// Forget a destroyed WebView's HWNDs so the wheel hook stops matching them.
///
/// Windows reuses HWND values: without this, a stale container entry could make
/// the low-level hook swallow wheel events over an unrelated window, and a
/// reused child HWND would be skipped by `subclass_one` (never re-subclassed).
#[cfg(windows)]
pub fn unregister_webview_container(container: isize) {
    WEBVIEW_CONTAINER_HWNDS.with(|s| {
        s.borrow_mut().remove(&container);
    });
    let subclassed = CONTAINER_SUBCLASSED_HWNDS.with(|m| m.borrow_mut().remove(&container));
    if let Some(subclassed) = subclassed {
        SUBCLASSED_HWNDS.with(|s| {
            let mut s = s.borrow_mut();
            for hwnd in subclassed {
                s.remove(&hwnd);
            }
        });
    }
}

#[cfg(not(windows))]
pub fn unregister_webview_container(_container: isize) {}

/// Whether partially visible embeds can be clipped with a window region.
///
/// When unsupported, callers must fall back to hiding the WebView unless the
/// embed is fully visible (an unclipped child HWND would paint over toolbars).
pub const fn region_clipping_supported() -> bool {
    cfg!(windows)
}

/// Clip the WebView container HWND to `rect` (container-local physical px).
///
/// `None` removes the region (fully visible embed). This lets a partially
/// scrolled embed keep showing the live player instead of swapping to the
/// thumbnail — the child HWND cannot otherwise be clipped by egui scroll areas.
#[cfg(windows)]
pub fn set_container_region(container: isize, rect: Option<(i32, i32, i32, i32)>) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateRectRgn, SetWindowRgn, HRGN};

    let hwnd = HWND(container as *mut _);
    unsafe {
        match rect {
            Some((left, top, right, bottom)) => {
                // SetWindowRgn takes ownership of the region on success.
                let region = CreateRectRgn(left, top, right, bottom);
                let _ = SetWindowRgn(hwnd, Some(region), true);
            }
            None => {
                let _ = SetWindowRgn(hwnd, None::<HRGN>, true);
            }
        }
    }
}

#[cfg(not(windows))]
pub fn set_container_region(_container: isize, _rect: Option<(i32, i32, i32, i32)>) {}
