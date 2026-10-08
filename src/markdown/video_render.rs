//! Video embed rendering for markdown documents.
//!
//! Trusted YouTube embeds use a wry child WebView positioned over the embed rect.
//! Any WebView failure, untrusted URL, or non-YouTube provider falls back to the
//! thumbnail path (YouTube CDN image + play affordance → system browser).

use super::parser::{VideoEmbedInfo, VideoProvider};
use super::video_webview_input::{drain_pending_wheel_into_egui, set_main_window_from_parent};
use eframe::egui::{
    self, Color32, ColorImage, CursorIcon, Id, LayerId, Pos2, Rect, Response, RichText, Sense,
    Shape, Stroke, TextureHandle, TextureOptions, Ui, Vec2,
};
use log::{error, warn};
use rust_i18n::t;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response as HttpResponse};
use wry::raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};
use wry::{NewWindowResponse, Rect as WryRect, WebView, WebViewBuilder, WebViewId};

/// Custom protocol for serving a same-origin HTML relay page that hosts the YouTube iframe.
/// WebView2 maps `ferrite-video://localhost/...` → `https://ferrite-video.localhost/...`.
const VIDEO_EMBED_PROTOCOL: &str = "ferrite-video";

/// Viewport height reserved for custom title bar — WebView2 must not keep focus here.
const TITLE_BAR_FOCUS_ZONE: f32 = 36.0;

/// Extra margin when testing foreground UI against embed rects (shadows / rounding).
pub(crate) const VIDEO_OCCLUDER_MARGIN: f32 = 20.0;

/// Colors used when drawing video embed fallbacks.
#[derive(Debug, Clone, Copy)]
pub struct VideoRenderColors {
    pub text: Color32,
    pub link: Color32,
    pub frame_border: Color32,
    pub frame_bg: Color32,
}

/// Borrowed parent window handle for wry child WebViews (cloned each frame).
#[derive(Clone, Copy)]
pub struct VideoWebViewParent {
    raw: RawWindowHandle,
}

// SAFETY: WebView parent handles are only captured and used on the UI thread
// during the same frame as the live parent window.
unsafe impl Send for VideoWebViewParent {}
unsafe impl Sync for VideoWebViewParent {}

impl VideoWebViewParent {
    /// Capture the native window handle from an eframe viewport.
    pub fn from_frame(frame: &eframe::Frame) -> Option<Self> {
        frame.window_handle().ok().map(|handle| Self {
            raw: handle.as_raw(),
        })
    }

    /// Win32 HWND for the parent window (wheel forwarding).
    #[cfg(windows)]
    pub(crate) fn win32_hwnd(&self) -> Option<isize> {
        use wry::raw_window_handle::RawWindowHandle;
        match self.raw {
            RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as isize),
            _ => None,
        }
    }
}

impl HasWindowHandle for VideoWebViewParent {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        // SAFETY: handle is only used synchronously on the UI thread during the
        // same frame it was captured from the live parent window.
        unsafe { Ok(WindowHandle::borrow_raw(self.raw)) }
    }
}

/// Per-frame render slot for syncing trusted embed WebViews during markdown render.
///
/// Installed by the app around `MarkdownEditor::show` on the UI thread only.
struct VideoWebViewRenderSlot {
    manager: *mut VideoWebViewManager,
    parent: VideoWebViewParent,
    key_prefix: String,
    pane_clip_rect: Rect,
    pixels_per_point: f32,
    ctx: egui::Context,
}

thread_local! {
    static VIDEO_WEBVIEW_RENDER_SLOT: RefCell<Option<VideoWebViewRenderSlot>> =
        const { RefCell::new(None) };
}

/// egui context for waking the event loop from WebView callbacks (wheel hook, IPC).
static VIDEO_REPAINT_CTX: OnceLock<egui::Context> = OnceLock::new();

/// Fullscreen state changes reported by relay pages via `window.ipc.postMessage`,
/// drained by the manager at `begin_frame` (keyed by embed key).
static PENDING_FULLSCREEN_EVENTS: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

/// Request an egui repaint from a non-frame context (hook or WebView callback).
pub(crate) fn request_video_repaint() {
    if let Some(ctx) = VIDEO_REPAINT_CTX.get() {
        ctx.request_repaint();
    }
}

fn push_fullscreen_event(embed_key: String, fullscreen: bool) {
    if let Ok(mut queue) = PENDING_FULLSCREEN_EVENTS.lock() {
        queue.push((embed_key, fullscreen));
    }
    request_video_repaint();
}

fn drain_fullscreen_events() -> Vec<(String, bool)> {
    PENDING_FULLSCREEN_EVENTS
        .lock()
        .map(|mut queue| queue.drain(..).collect())
        .unwrap_or_default()
}

/// Install the active WebView render slot for the current UI frame.
///
/// Must be paired with [`pop_video_webview_render_slot`] after `MarkdownEditor::show`.
pub fn push_video_webview_render_slot(
    manager: &mut VideoWebViewManager,
    parent: VideoWebViewParent,
    ctx: &egui::Context,
    key_prefix: String,
    pane_clip_rect: Rect,
    pixels_per_point: f32,
    focus_priority_rects: Vec<Rect>,
) {
    let manager = std::ptr::from_mut(manager);
    VIDEO_WEBVIEW_RENDER_SLOT.with(|slot| {
        // SAFETY: manager pointer is valid until pop on the UI thread.
        unsafe {
            (*manager).begin_frame(focus_priority_rects);
        }
        let _ = VIDEO_REPAINT_CTX.set(ctx.clone());
        set_main_window_from_parent(&parent);
        drain_pending_wheel_into_egui(ctx);
        *slot.borrow_mut() = Some(VideoWebViewRenderSlot {
            manager,
            parent,
            key_prefix,
            pane_clip_rect,
            pixels_per_point,
            ctx: ctx.clone(),
        });
    });
}

/// Clear the active WebView render slot and finalize embed cleanup for the frame.
pub fn pop_video_webview_render_slot() {
    VIDEO_WEBVIEW_RENDER_SLOT.with(|slot| {
        if let Some(active) = slot.borrow_mut().take() {
            // SAFETY: slot is only set/cleared on the UI thread around editor show.
            unsafe {
                (*active.manager).end_frame(&active.ctx);
            }
        }
    });
}

fn with_render_slot<R>(
    f: impl FnOnce(&mut VideoWebViewManager, &VideoWebViewParent, &str, Rect, f32) -> R,
) -> Option<R> {
    VIDEO_WEBVIEW_RENDER_SLOT.with(|slot| {
        let mut guard = slot.borrow_mut();
        let active = guard.as_mut()?;
        // SAFETY: slot lifetime is bounded by push/pop on the UI thread.
        let manager = unsafe { &mut *active.manager };
        Some(f(
            manager,
            &active.parent,
            &active.key_prefix,
            active.pane_clip_rect,
            active.pixels_per_point,
        ))
    })
}

/// Manages wry child WebViews for visible trusted video embeds.
pub struct VideoWebViewManager {
    webviews: HashMap<String, ActiveWebView>,
    seen_this_frame: HashSet<String>,
    /// Embed keys whose WebView failed to create; avoids per-frame recreate storms.
    failed_embeds: HashSet<String>,
    /// Viewport rects of synced embeds this frame (keyed by embed key).
    embed_screen_rects_this_frame: HashMap<String, Rect>,
    /// Visible (clip-intersected) portion of each synced embed this frame, in
    /// screen space. Drives show/hide and window-region clipping.
    embed_visible_rects_this_frame: HashMap<String, Rect>,
    /// Ferrite UI rects that must receive input instead of the WebView (e.g. split raw pane).
    focus_priority_rects: Vec<Rect>,
    /// Foreground overlay rects from the last occlusion pass (screen space).
    foreground_occluders: Vec<Rect>,
    /// Whether focus was already handed back to Ferrite for the current
    /// pointer-outside-embeds state (edge-triggered, not per-frame).
    focus_yielded: bool,
    /// When each unsynced WebView first went stale; destroyed after
    /// [`WEBVIEW_STALE_GRACE`] so scrolling back does not pay creation cost again.
    stale_since: HashMap<String, Instant>,
    /// Embeds whose relay page currently has a fullscreen element (via IPC).
    fullscreen_embeds: HashSet<String>,
    force_fallback: bool,
}

/// How long a scrolled-out WebView is kept alive (hidden) before destruction.
/// WebView2 creation costs ~100ms; destroying on the first unseen frame made
/// scroll-back stutter and restarted playback.
const WEBVIEW_STALE_GRACE: Duration = Duration::from_millis(2500);

struct ActiveWebView {
    webview: WebView,
    loaded_url: String,
    /// Last visibility applied via `set_visible` (avoids per-frame churn).
    visible: bool,
    /// Last screen-space rect applied via `set_bounds` (avoids per-frame churn).
    last_global_rect: Rect,
    /// Last window-region clip applied (container-local physical px); `None` = unclipped.
    last_region: Option<(i32, i32, i32, i32)>,
    /// Win32 container HWND for wheel-hook cleanup on destruction.
    container_hwnd: Option<isize>,
}

impl Default for VideoWebViewManager {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoWebViewManager {
    pub fn new() -> Self {
        Self {
            webviews: HashMap::new(),
            seen_this_frame: HashSet::new(),
            failed_embeds: HashSet::new(),
            embed_screen_rects_this_frame: HashMap::new(),
            embed_visible_rects_this_frame: HashMap::new(),
            focus_priority_rects: Vec::new(),
            foreground_occluders: Vec::new(),
            focus_yielded: false,
            stale_since: HashMap::new(),
            fullscreen_embeds: HashSet::new(),
            force_fallback: false,
        }
    }

    /// Mark the start of a rendered-view frame (clears the active-embed set).
    pub fn begin_frame(&mut self, focus_priority_rects: Vec<Rect>) {
        self.seen_this_frame.clear();
        self.embed_screen_rects_this_frame.clear();
        self.embed_visible_rects_this_frame.clear();
        self.focus_priority_rects = focus_priority_rects;

        // Apply fullscreen state reported by relay pages since the last frame.
        for (key, fullscreen) in drain_fullscreen_events() {
            if fullscreen && self.webviews.contains_key(&key) {
                self.fullscreen_embeds.insert(key);
            } else {
                self.fullscreen_embeds.remove(&key);
            }
        }
        // Prune fullscreen keys whose WebView is gone (stale IPC).
        self.fullscreen_embeds
            .retain(|key| self.webviews.contains_key(key));
        super::video_webview_input::set_video_fullscreen_active(!self.fullscreen_embeds.is_empty());
    }

    /// Screen rects of synced embeds this frame (for wheel-hook hit testing).
    pub fn embed_screen_rects(&self) -> Vec<Rect> {
        self.embed_screen_rects_this_frame
            .values()
            .copied()
            .collect()
    }

    /// Drop WebViews that were not synced this frame; return focus to Ferrite when appropriate.
    pub fn end_frame(&mut self, ctx: &egui::Context) {
        let pointer_pos = ctx.input(|i| i.pointer.interact_pos());
        let viewport = ctx.content_rect();
        let mut priority = self.focus_priority_rects.clone();
        priority.push(Rect::from_min_max(
            viewport.min,
            Pos2::new(viewport.max.x, viewport.min.y + TITLE_BAR_FOCUS_ZONE),
        ));

        // Edge-triggered: yield once when the pointer leaves the embeds, not every
        // frame. Per-frame `focus_parent` calls interfered with in-flight scroll and
        // caused visible jitter while WebViews were on screen.
        let embed_rects: Vec<Rect> = self
            .embed_screen_rects_this_frame
            .values()
            .copied()
            .collect();
        let yield_focus = should_yield_focus_to_ferrite(pointer_pos, &embed_rects, &priority);
        if yield_focus && !self.focus_yielded {
            for entry in self.webviews.values() {
                let _ = entry.webview.focus_parent();
            }
        }
        self.focus_yielded = yield_focus;

        // Unsynced WebViews are hidden immediately but destroyed only after a grace
        // period, so scrolling out and back does not restart the player. Fullscreen
        // embeds are exempt — their layout rect may be scrolled away while the
        // window-sized player is up.
        let now = Instant::now();
        let stale: Vec<String> = self
            .webviews
            .keys()
            .filter(|key| {
                !self.seen_this_frame.contains(*key) && !self.fullscreen_embeds.contains(*key)
            })
            .cloned()
            .collect();
        for key in stale {
            let first_stale = *self.stale_since.entry(key.clone()).or_insert(now);
            if now.duration_since(first_stale) > WEBVIEW_STALE_GRACE {
                self.drop_webview_entry(&key);
                continue;
            }
            if let Some(entry) = self.webviews.get_mut(&key) {
                if entry.visible {
                    let _ = entry.webview.focus_parent();
                    if entry.webview.set_visible(false).is_ok() {
                        entry.visible = false;
                    }
                }
            }
        }
        self.stale_since
            .retain(|key, _| !self.seen_this_frame.contains(key));

        // Deliberately keep `embed_screen_rects_this_frame`: the foreground-occlusion
        // pass (`apply_foreground_occlusion`) runs after end_frame, once dialogs and
        // overlays have rendered, and needs these rects for intersection tests.
        // The map is cleared at the start of the next rendered frame in `begin_frame`.
    }

    /// Destroy one WebView and unregister its HWNDs from the wheel-forwarding hook.
    fn drop_webview_entry(&mut self, key: &str) {
        if let Some(entry) = self.webviews.remove(key) {
            let _ = entry.webview.focus_parent();
            if let Some(container) = entry.container_hwnd {
                super::video_webview_input::unregister_webview_container(container);
            }
        }
        self.stale_since.remove(key);
        self.fullscreen_embeds.remove(key);
    }

    /// Hide an embed's WebView immediately so egui can receive resize-handle input.
    ///
    /// WebView2 child HWNDs sit above the glow surface; without this, the bottom-right
    /// resize grip never receives hover/drag until `end_frame` drops the stale WebView.
    pub fn suppress_embed_for_resize(&mut self, embed_key: &str) {
        if let Some(entry) = self.webviews.get_mut(embed_key) {
            if entry.visible {
                let _ = entry.webview.focus_parent();
                if entry.webview.set_visible(false).is_ok() {
                    entry.visible = false;
                }
            }
        }
    }

    /// Destroy every active child WebView (tab switch, Raw mode, inactive tab, etc.).
    pub fn clear_all(&mut self) {
        let keys: Vec<String> = self.webviews.keys().cloned().collect();
        for key in keys {
            self.drop_webview_entry(&key);
        }
        self.seen_this_frame.clear();
        self.failed_embeds.clear();
        self.embed_screen_rects_this_frame.clear();
        self.embed_visible_rects_this_frame.clear();
        self.stale_since.clear();
        self.fullscreen_embeds.clear();
        super::video_webview_input::set_video_fullscreen_active(false);
    }

    /// Hide native WebViews only where foreground egui UI overlaps the embed rect.
    ///
    /// WebView2 child HWNDs sit above the glow surface. Full-window hide made videos
    /// vanish when opening small overlays (e.g. quick switcher) that do not cover them.
    pub fn apply_foreground_occlusion(&mut self, occluders: &[Rect]) {
        self.foreground_occluders = occluders.to_vec();
        let keys: Vec<String> = self.webviews.keys().cloned().collect();
        for key in keys {
            self.apply_visibility_for_key(&key);
        }
    }

    fn embed_obscured(&self, embed_key: &str) -> bool {
        let Some(embed_rect) = self.embed_screen_rects_this_frame.get(embed_key) else {
            return false;
        };
        embed_rect_intersects_occluders(*embed_rect, &self.foreground_occluders)
    }

    fn apply_visibility_for_key(&mut self, embed_key: &str) {
        let fullscreen = self.fullscreen_embeds.contains(embed_key);
        let show = fullscreen || self.embed_should_show(embed_key);
        let Some(entry) = self.webviews.get_mut(embed_key) else {
            return;
        };
        // Only act on transitions — re-applying `set_visible`/`focus_parent` every
        // frame caused flicker and stole in-flight scroll/keyboard input.
        if show == entry.visible {
            return;
        }
        if !show {
            let _ = entry.webview.focus_parent();
        }
        if entry.webview.set_visible(show).is_ok() {
            entry.visible = show;
        }
    }

    /// Whether a (non-fullscreen) embed's WebView should be visible this frame.
    ///
    /// With window-region clipping, any usable visible portion shows the live
    /// player (clipped to the pane). Without it, only fully visible embeds show —
    /// an unclipped child HWND would paint over surrounding UI.
    fn embed_should_show(&self, embed_key: &str) -> bool {
        let Some(visible) = self.embed_visible_rects_this_frame.get(embed_key) else {
            return false;
        };
        if visible.width() < MIN_WEBVIEW_VISIBLE_SIZE || visible.height() < MIN_WEBVIEW_VISIBLE_SIZE
        {
            return false;
        }
        if self.embed_obscured(embed_key) {
            return false;
        }
        if super::video_webview_input::region_clipping_supported() {
            return true;
        }
        let Some(full) = self.embed_screen_rects_this_frame.get(embed_key) else {
            return false;
        };
        embed_rect_fully_visible_in(*visible, *full)
    }

    /// When true, all embed sync attempts fail (for fallback testing).
    #[cfg(test)]
    pub fn set_force_fallback(&mut self, force: bool) {
        self.force_fallback = force;
    }

    /// Whether this manager would attempt a WebView for `info` (gate + no forced fallback).
    pub fn would_use_webview(&self, info: &VideoEmbedInfo) -> bool {
        !self.force_fallback && Self::is_webview_eligible(info)
    }

    /// Single gate for the WebView path — only trusted embeds with a provider URL.
    pub fn is_webview_eligible(info: &VideoEmbedInfo) -> bool {
        info.trusted && provider_embed_url(info).is_some()
    }

    /// Sync a trusted embed WebView over `rect`, returning true when active.
    ///
    /// `visible_rect` is the clip-intersected portion of `rect` (same layer space);
    /// it drives show/hide and the window-region clip for partial visibility.
    #[allow(clippy::too_many_arguments)]
    pub fn sync_trusted_embed(
        &mut self,
        parent: &VideoWebViewParent,
        embed_key: &str,
        info: &VideoEmbedInfo,
        rect: Rect,
        visible_rect: Rect,
        layer_id: LayerId,
        ctx: &egui::Context,
        pixels_per_point: f32,
    ) -> bool {
        if self.force_fallback || !Self::is_webview_eligible(info) {
            return false;
        }

        if self.failed_embeds.contains(embed_key) {
            return false;
        }

        let Some(url) = provider_relay_page_url(info) else {
            return false;
        };

        let fullscreen = self.fullscreen_embeds.contains(embed_key);
        let (global_rect, visible_global) = if fullscreen {
            // Window-fullscreen: the player covers the whole viewport regardless
            // of where the embed's layout rect currently is.
            let full = ctx.viewport_rect();
            (full, full)
        } else {
            (
                rect_to_viewport(ctx, layer_id, rect),
                rect_to_viewport(ctx, layer_id, visible_rect),
            )
        };
        if !global_rect.is_positive() {
            return false;
        }
        let bounds = wry_bounds_from_global_rect(global_rect);

        self.seen_this_frame.insert(embed_key.to_string());
        self.embed_screen_rects_this_frame
            .insert(embed_key.to_string(), global_rect);
        self.embed_visible_rects_this_frame
            .insert(embed_key.to_string(), visible_global);

        let existing_sync = if let Some(entry) = self.webviews.get_mut(embed_key) {
            if entry.loaded_url != url {
                false
            } else if rects_approx_eq(entry.last_global_rect, global_rect) {
                // Steady state: bounds unchanged — skip the native `set_bounds` call.
                true
            } else if entry.webview.set_bounds(bounds).is_ok() {
                entry.last_global_rect = global_rect;
                true
            } else {
                false
            }
        } else {
            false
        };

        if self.webviews.contains_key(embed_key) && !existing_sync {
            self.drop_webview_entry(embed_key);
            return false;
        }

        if existing_sync {
            self.sync_container_region(embed_key, global_rect, visible_global, pixels_per_point);
            self.apply_visibility_for_key(embed_key);
            if let Some(entry) = self.webviews.get(embed_key) {
                // Re-walk each frame: WebView2 creates child HWNDs asynchronously.
                let _ = super::video_webview_input::install_wheel_forwarding(&entry.webview);
            }
            return true;
        }

        let video_id = info.video_id.as_deref().unwrap_or_default();
        match create_child_webview(parent, embed_key, video_id, &url, bounds) {
            Ok(webview) => {
                // WebView2 can grab keyboard focus during creation even with
                // `with_focused(false)`. Creation happens exactly when an embed
                // scrolls fully into view — hand focus straight back so scrolling
                // and shortcuts keep working.
                let _ = webview.focus_parent();
                let container_hwnd = super::video_webview_input::install_wheel_forwarding(&webview);
                self.webviews.insert(
                    embed_key.to_string(),
                    ActiveWebView {
                        webview,
                        loaded_url: url,
                        visible: true,
                        last_global_rect: global_rect,
                        last_region: None,
                        container_hwnd,
                    },
                );
                self.sync_container_region(
                    embed_key,
                    global_rect,
                    visible_global,
                    pixels_per_point,
                );
                self.apply_visibility_for_key(embed_key);
                true
            }
            Err(()) => {
                self.failed_embeds.insert(embed_key.to_string());
                false
            }
        }
    }

    /// Clip the WebView container to the embed's visible portion (Windows only).
    ///
    /// Partially scrolled embeds keep showing the live player instead of swapping
    /// to the thumbnail; the region prevents the child HWND from painting over
    /// toolbars and neighbouring panes. No-op when the region is unchanged.
    fn sync_container_region(
        &mut self,
        embed_key: &str,
        global_rect: Rect,
        visible_global: Rect,
        pixels_per_point: f32,
    ) {
        if !super::video_webview_input::region_clipping_supported() {
            return;
        }
        let Some(entry) = self.webviews.get_mut(embed_key) else {
            return;
        };
        let Some(container) = entry.container_hwnd else {
            return;
        };
        let region = compute_container_region(global_rect, visible_global, pixels_per_point);
        if entry.last_region == region {
            return;
        }
        super::video_webview_input::set_container_region(container, region);
        entry.last_region = region;
    }
}

/// Container-local physical-pixel clip for the visible part of an embed.
/// Returns `None` when the embed is (effectively) fully visible.
fn compute_container_region(
    global_rect: Rect,
    visible_global: Rect,
    pixels_per_point: f32,
) -> Option<(i32, i32, i32, i32)> {
    if embed_rect_fully_visible_in(visible_global, global_rect) {
        return None;
    }
    let ppp = if pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    let clipped = visible_global.intersect(global_rect);
    let left = ((clipped.min.x - global_rect.min.x) * ppp).floor().max(0.0) as i32;
    let top = ((clipped.min.y - global_rect.min.y) * ppp).floor().max(0.0) as i32;
    let right = ((clipped.max.x - global_rect.min.x) * ppp).ceil() as i32;
    let bottom = ((clipped.max.y - global_rect.min.y) * ppp).ceil() as i32;
    Some((left, top, right.max(left), bottom.max(top)))
}

/// Screen-space rect comparison tolerance for skipping redundant `set_bounds` calls.
const BOUNDS_EPSILON: f32 = 0.1;

fn rects_approx_eq(a: Rect, b: Rect) -> bool {
    (a.min.x - b.min.x).abs() < BOUNDS_EPSILON
        && (a.min.y - b.min.y).abs() < BOUNDS_EPSILON
        && (a.max.x - b.max.x).abs() < BOUNDS_EPSILON
        && (a.max.y - b.max.y).abs() < BOUNDS_EPSILON
}

fn is_valid_youtube_video_id(video_id: &str) -> bool {
    !video_id.is_empty()
        && video_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn video_id_from_relay_uri(uri: &str) -> Option<String> {
    let parsed = url::Url::parse(uri).ok()?;
    if let Some((_, value)) = parsed.query_pairs().find(|(key, _)| key == "v") {
        if !value.is_empty() {
            return Some(value.into_owned());
        }
    }
    None
}

/// Relay page URL loaded by the child WebView (not the raw YouTube `/embed/` URL).
fn provider_relay_page_url(info: &VideoEmbedInfo) -> Option<String> {
    if !info.trusted || info.provider != VideoProvider::YouTube {
        return None;
    }
    let video_id = info.video_id.as_deref()?;
    if !is_valid_youtube_video_id(video_id) {
        return None;
    }
    Some(format!(
        "{VIDEO_EMBED_PROTOCOL}://localhost/embed?v={video_id}"
    ))
}

fn youtube_embed_relay_html(video_id: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta name="referrer" content="strict-origin-when-cross-origin" />
  <style>
    html, body {{ height: 100%; margin: 0; background: #000; overflow: hidden; }}
    iframe {{ width: 100%; height: 100%; border: 0; display: block; }}
  </style>
</head>
<body>
  <iframe
    id="player"
    allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; fullscreen"
    allowfullscreen
    referrerpolicy="strict-origin-when-cross-origin"
  ></iframe>
  <script>
    (function() {{
      var v = "{video_id}";
      var base = "https://www.youtube-nocookie.com/embed/" + encodeURIComponent(v);
      var params = new URLSearchParams({{
        enablejsapi: "1",
        rel: "0",
        modestbranding: "1",
        playsinline: "1",
        origin: location.origin
      }});
      document.getElementById("player").src = base + "?" + params.toString();

      // Fullscreen propagates from the player iframe to this top document.
      // Report it to the host so the WebView bounds can cover the window.
      document.addEventListener("fullscreenchange", function() {{
        if (window.ipc && window.ipc.postMessage) {{
          window.ipc.postMessage(
            document.fullscreenElement ? "fullscreen:on" : "fullscreen:off"
          );
        }}
      }});
    }})();
  </script>
</body>
</html>"#
    )
}

fn serve_youtube_embed_relay(
    _id: WebViewId,
    request: Request<Vec<u8>>,
) -> HttpResponse<Cow<'static, [u8]>> {
    let uri = request.uri().to_string();
    let video_id = video_id_from_relay_uri(&uri).unwrap_or_default();
    let html = if is_valid_youtube_video_id(&video_id) {
        youtube_embed_relay_html(&video_id)
    } else {
        "<html><body>Invalid video id</body></html>".to_string()
    };

    HttpResponse::builder()
        .header(CONTENT_TYPE, "text/html; charset=utf-8")
        .status(200)
        .body(Cow::Owned(html.into_bytes()))
        .unwrap_or_else(|_| {
            HttpResponse::builder()
                .status(500)
                .body(Cow::Borrowed(&b"relay error"[..]))
                .expect("static error response")
        })
}

/// Navigation allowlist for the embed WebView.
///
/// Only the local relay page and the YouTube embed player origin may load.
/// WebView2 routes top-level navigations through this handler; WKWebView also
/// routes iframe navigations, so the player origin must be allowed explicitly.
/// On Windows/Android, wry surfaces the custom protocol as
/// `{http,https}://ferrite-video.localhost/...`.
fn is_allowed_webview_navigation(url: &str) -> bool {
    url.starts_with("ferrite-video://")
        || url.starts_with("http://ferrite-video.localhost")
        || url.starts_with("https://ferrite-video.localhost")
        || url.starts_with("https://www.youtube-nocookie.com/")
        || url == "about:blank"
}

fn create_child_webview(
    parent: &VideoWebViewParent,
    embed_key: &str,
    video_id: &str,
    relay_url: &str,
    bounds: WryRect,
) -> Result<WebView, ()> {
    if !is_valid_youtube_video_id(video_id) {
        return Err(());
    }

    let ipc_embed_key = embed_key.to_string();
    WebViewBuilder::new()
        .with_custom_protocol(VIDEO_EMBED_PROTOCOL.to_string(), serve_youtube_embed_relay)
        .with_url(relay_url)
        .with_bounds(bounds)
        .with_focused(false)
        // The relay page reports fullscreen transitions; the manager resizes the
        // WebView to cover the window while a fullscreen element is active.
        .with_ipc_handler(move |request| match request.body().as_str() {
            "fullscreen:on" => push_fullscreen_event(ipc_embed_key.clone(), true),
            "fullscreen:off" => push_fullscreen_event(ipc_embed_key.clone(), false),
            other => warn!("Ignoring unknown video embed IPC message: '{}'", other),
        })
        .with_navigation_handler(|url| {
            let allowed = is_allowed_webview_navigation(&url);
            if !allowed {
                warn!("Blocked video WebView navigation to '{}'", url);
            }
            allowed
        })
        // Popup requests (e.g. "Watch on YouTube") open in the system browser;
        // the embed WebView never spawns new native windows.
        .with_new_window_req_handler(|url, _features| {
            if url.starts_with("https://") || url.starts_with("http://") {
                if let Err(e) = open::that(&url) {
                    error!("Failed to open video link in browser '{}': {}", url, e);
                }
            }
            NewWindowResponse::Deny
        })
        .build_as_child(parent)
        .map_err(|e| {
            warn!(
                "Failed to create video WebView for relay '{}': {}",
                relay_url, e
            );
        })
}

/// Minimum visible dimension (logical px) before a WebView overlay is shown.
const MIN_WEBVIEW_VISIBLE_SIZE: f32 = 2.0;

/// Tolerance when comparing a clip intersection to the full embed rect (logical px).
const FULL_VISIBILITY_EPSILON: f32 = 0.5;

/// Returns the full layout rect when any part of the embed is on-screen.
///
/// Bounds are always the full 16:9 slot — never the scroll intersection (that squashes
/// the iframe). When only partially visible, the caller hides the WebView and the egui
/// thumbnail underlay fills the clipped slot.
fn embed_rect_for_webview(ui: &Ui, embed_rect: Rect, pane_clip_rect: Rect) -> Option<Rect> {
    if !ui.is_rect_visible(embed_rect) {
        return None;
    }

    let visible = embed_rect
        .intersect(ui.clip_rect())
        .intersect(pane_clip_rect);
    if visible.width() < MIN_WEBVIEW_VISIBLE_SIZE || visible.height() < MIN_WEBVIEW_VISIBLE_SIZE {
        return None;
    }

    Some(embed_rect)
}

/// Clip-intersected portion of the embed rect (layer space).
fn embed_visible_portion(ui: &Ui, embed_rect: Rect, pane_clip_rect: Rect) -> Rect {
    embed_rect
        .intersect(ui.clip_rect())
        .intersect(pane_clip_rect)
}

fn embed_rect_fully_visible_in(visible: Rect, embed_rect: Rect) -> bool {
    (visible.min.x - embed_rect.min.x).abs() <= FULL_VISIBILITY_EPSILON
        && (visible.min.y - embed_rect.min.y).abs() <= FULL_VISIBILITY_EPSILON
        && (visible.max.x - embed_rect.max.x).abs() <= FULL_VISIBILITY_EPSILON
        && (visible.max.y - embed_rect.max.y).abs() <= FULL_VISIBILITY_EPSILON
}

/// Convert a widget rect in layer space to screen/viewport coordinates.
pub(crate) fn egui_rect_to_screen(ctx: &egui::Context, layer_id: LayerId, rect: Rect) -> Rect {
    if let Some(to_global) = ctx.layer_transform_to_global(layer_id) {
        to_global * rect
    } else {
        rect
    }
}

fn rect_to_viewport(ctx: &egui::Context, layer_id: LayerId, rect: Rect) -> Rect {
    egui_rect_to_screen(ctx, layer_id, rect)
}

fn embed_rect_intersects_occluders(embed_rect: Rect, occluders: &[Rect]) -> bool {
    const MARGIN: f32 = 2.0;
    occluders
        .iter()
        .any(|occluder| embed_rect.intersects(occluder.expand(MARGIN)))
}

fn wry_bounds_from_global_rect(global_rect: Rect) -> WryRect {
    WryRect {
        position: LogicalPosition::new(global_rect.min.x, global_rect.min.y).into(),
        size: LogicalSize::new(global_rect.width(), global_rect.height()).into(),
    }
}

/// When true, HWND focus should return to the parent window so egui can handle input.
fn should_yield_focus_to_ferrite(
    pointer_pos: Option<Pos2>,
    embed_screen_rects: &[Rect],
    focus_priority_rects: &[Rect],
) -> bool {
    let Some(pos) = pointer_pos else {
        return true;
    };
    if focus_priority_rects.iter().any(|rect| rect.contains(pos)) {
        return true;
    }
    !embed_screen_rects.iter().any(|rect| rect.contains(pos))
}

/// YouTube thumbnail quality suffix used for embed previews.
const YOUTUBE_THUMBNAIL_SUFFIX: &str = "hqdefault.jpg";

const EMBED_ASPECT_RATIO: f32 = 9.0 / 16.0;

/// Minimum logical size while drag-resizing a video embed.
const MIN_EMBED_RESIZE_WIDTH: f32 = 160.0;
const MIN_EMBED_RESIZE_HEIGHT: f32 = 90.0;

/// Drag strip below the video (outside the WebView overlay — always reachable by egui).
const VIDEO_RESIZE_BAR_HEIGHT: f32 = 18.0;

/// Source line (1-indexed) for drag-resize write-back (see [`VideoEmbedResizeCommit`]).
#[derive(Clone, Copy)]
pub struct VideoEmbedResizeContext {
    pub source_line: usize,
    /// When false, the resize handle is not shown (e.g. preview-locked).
    pub enabled: bool,
}

impl VideoEmbedResizeContext {
    pub fn new(source_line: usize, enabled: bool) -> Self {
        Self {
            source_line,
            enabled,
        }
    }
}

/// Dimensions to persist into the `{{video …}}` source on drag release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoEmbedResizeCommit {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy)]
struct VideoEmbedResizeDrag {
    start_width: f32,
    start_height: f32,
}

#[derive(Clone, Copy)]
struct VideoEmbedPendingSize {
    width: f32,
    height: f32,
}

fn video_embed_resize_ids(source_line: usize, url: &str) -> Id {
    Id::new("video_embed_resize").with(source_line).with(url)
}

fn clamp_display_size(size: Vec2, available_width: f32) -> Vec2 {
    if size.x > available_width && available_width > 0.0 {
        let scale = available_width / size.x;
        Vec2::new(available_width, size.y * scale)
    } else {
        size
    }
}

fn pending_video_display_size(
    ui: &Ui,
    info: &VideoEmbedInfo,
    available_width: f32,
    source_line: usize,
) -> Option<Vec2> {
    let base_id = video_embed_resize_ids(source_line, &info.url);
    // Pending size is only for live drag feedback — never override parsed dimensions.
    if ui
        .data(|d| d.get_temp::<VideoEmbedResizeDrag>(base_id.with("drag")))
        .is_none()
    {
        return None;
    }
    let pending_id = base_id.with("pending");
    let pending = ui.data(|d| d.get_temp::<VideoEmbedPendingSize>(pending_id))?;
    Some(clamp_display_size(
        Vec2::new(pending.width, pending.height),
        available_width,
    ))
}

/// Advance the running drag size by one frame's pointer delta.
///
/// `drag_delta` is the per-frame pointer movement, so it must be added to the previous
/// pending size (not the drag-start size) to accumulate the full drag distance.
fn accumulate_resize(base: VideoEmbedPendingSize, frame_delta: Vec2) -> VideoEmbedPendingSize {
    VideoEmbedPendingSize {
        width: (base.width + frame_delta.x).max(MIN_EMBED_RESIZE_WIDTH),
        height: (base.height + frame_delta.y).max(MIN_EMBED_RESIZE_HEIGHT),
    }
}

fn embed_resize_drag_active(ui: &Ui, info: &VideoEmbedInfo, source_line: usize) -> bool {
    let drag_id = video_embed_resize_ids(source_line, &info.url).with("drag");
    ui.data(|d| d.get_temp::<VideoEmbedResizeDrag>(drag_id))
        .is_some()
}

fn paint_resize_bar(ui: &Ui, bar_rect: Rect, hovered: bool, dragged: bool) {
    let painter = ui.painter();
    let alpha = if dragged {
        120
    } else if hovered {
        90
    } else {
        55
    };
    painter.rect_filled(bar_rect, 3.0, Color32::from_black_alpha(alpha));

    let grip_center = bar_rect.right_center() - Vec2::new(14.0, 0.0);
    for i in 0..3 {
        let offset = i as f32 * 4.0;
        painter.line_segment(
            [
                grip_center + Vec2::new(offset - 6.0, -4.0),
                grip_center + Vec2::new(offset + 2.0, 4.0),
            ],
            Stroke::new(1.5, Color32::from_white_alpha(210)),
        );
    }
}

fn handle_video_embed_resize(
    ui: &mut Ui,
    info: &VideoEmbedInfo,
    video_rect: Rect,
    resize_bar_rect: Rect,
    source_line: usize,
) -> Option<VideoEmbedResizeCommit> {
    let base_id = video_embed_resize_ids(source_line, &info.url);
    let bar_id = base_id.with("bar");
    let drag_id = base_id.with("drag");
    let pending_id = base_id.with("pending");

    // Full-width strip below the WebView — always egui-reachable (no HWND overlap).
    let response = ui.interact(resize_bar_rect, bar_id, Sense::click_and_drag());
    let hovered = response.hovered();
    let dragged = response.dragged();
    let drag_started = response.drag_started();
    let drag_stopped = response.drag_stopped();
    let drag_delta = response.drag_delta();

    paint_resize_bar(ui, resize_bar_rect, hovered, dragged);
    let _ = response
        .on_hover_cursor(CursorIcon::ResizeNwSe)
        .on_hover_text(t!("markdown.video_embed.resize_tooltip").to_string());

    if drag_started {
        ui.data_mut(|d| {
            d.insert_temp(
                drag_id,
                VideoEmbedResizeDrag {
                    start_width: video_rect.width(),
                    start_height: video_rect.height(),
                },
            );
        });
    }

    if dragged {
        if let Some(drag) = ui.data(|d| d.get_temp::<VideoEmbedResizeDrag>(drag_id)) {
            // Accumulate the drag: `drag_delta()` is this frame's pointer movement only,
            // so we add it to the running pending size (seeded from the drag-start size on
            // the first frame). Using `start_* + drag_delta` would reset every frame and the
            // embed would snap back to its original size on release.
            let base = ui
                .data(|d| d.get_temp::<VideoEmbedPendingSize>(pending_id))
                .unwrap_or(VideoEmbedPendingSize {
                    width: drag.start_width,
                    height: drag.start_height,
                });
            let next = accumulate_resize(base, drag_delta);
            ui.data_mut(|d| d.insert_temp(pending_id, next));
            ui.ctx().request_repaint();
        }
    }

    if drag_stopped {
        ui.data_mut(|d| d.remove::<VideoEmbedResizeDrag>(drag_id));
        if let Some(pending) = ui.data(|d| d.get_temp::<VideoEmbedPendingSize>(pending_id)) {
            ui.data_mut(|d| d.remove::<VideoEmbedPendingSize>(pending_id));
            let width = pending.width.round().clamp(
                MIN_VIDEO_EMBED_DIMENSION as f32,
                MAX_VIDEO_EMBED_DIMENSION as f32,
            ) as u32;
            let height = pending.height.round().clamp(
                MIN_VIDEO_EMBED_DIMENSION as f32,
                MAX_VIDEO_EMBED_DIMENSION as f32,
            ) as u32;
            return Some(VideoEmbedResizeCommit { width, height });
        }
    }

    None
}

const MIN_VIDEO_EMBED_DIMENSION: u32 = 1;
const MAX_VIDEO_EMBED_DIMENSION: u32 = 8192;

#[derive(Clone)]
struct CachedVideoThumbnail {
    texture: TextureHandle,
    width: u32,
    height: u32,
}

#[derive(Clone)]
enum VideoThumbnailCacheEntry {
    /// Fetch in flight on a background thread; a placeholder renders meanwhile.
    Loading,
    Loaded(CachedVideoThumbnail),
    Failed,
}

/// Whether a trusted embed has enough metadata for the WebView relay path.
pub fn provider_embed_url(info: &VideoEmbedInfo) -> Option<String> {
    provider_relay_page_url(info)
}

/// Build the YouTube thumbnail URL for a parsed embed, if applicable.
pub fn youtube_thumbnail_url(info: &VideoEmbedInfo) -> Option<String> {
    if info.provider != VideoProvider::YouTube {
        return None;
    }
    let video_id = info.video_id.as_deref()?;
    if video_id.is_empty() {
        return None;
    }
    Some(format!(
        "https://img.youtube.com/vi/{}/{}",
        video_id, YOUTUBE_THUMBNAIL_SUFFIX
    ))
}

fn embed_stable_key(info: &VideoEmbedInfo) -> String {
    if let Some(id) = info.video_id.as_deref() {
        if !id.is_empty() {
            return id.to_string();
        }
    }
    info.url.clone()
}

/// Unique per-occurrence WebView key.
///
/// Must include the source line: a document can embed the *same* video several
/// times, and keying by video id alone made all occurrences share one WebView —
/// its bounds were re-set once per occurrence per frame, so the single HWND
/// visibly jumped between embed slots ("doubling"/flicker).
fn embed_webview_key(key_prefix: &str, info: &VideoEmbedInfo, source_line: usize) -> String {
    format!("{}:{}:{}", key_prefix, embed_stable_key(info), source_line)
}

fn video_display_size(info: &VideoEmbedInfo, available_width: f32) -> Vec2 {
    let available_width = available_width.max(1.0);

    let (target_w, target_h) = match (info.width, info.height) {
        (Some(w), Some(h)) => (w as f32, h as f32),
        (Some(w), None) => {
            let width = w as f32;
            (width, width * EMBED_ASPECT_RATIO)
        }
        (None, Some(h)) => {
            let height = h as f32;
            (height / EMBED_ASPECT_RATIO, height)
        }
        (None, None) => (available_width, available_width * EMBED_ASPECT_RATIO),
    };

    if target_w > available_width {
        let scale = available_width / target_w;
        Vec2::new(available_width, target_h * scale)
    } else {
        Vec2::new(target_w, target_h)
    }
}

/// Render a video embed, preferring the WebView path for trusted embeds when context is set.
///
/// When `resize` is enabled, a bottom-right handle allows drag-resizing; on release returns
/// [`VideoEmbedResizeCommit`] so the caller can write dimensions into the markdown source.
pub fn render_video_embed(
    ui: &mut Ui,
    info: &VideoEmbedInfo,
    colors: &VideoRenderColors,
    font_size: f32,
    resize: Option<VideoEmbedResizeContext>,
) -> Option<VideoEmbedResizeCommit> {
    let available_width = ui.available_width();
    let resize_enabled = resize.is_some_and(|c| c.enabled);
    let video_size = if let Some(ctx) = resize.filter(|c| c.enabled) {
        pending_video_display_size(ui, info, available_width, ctx.source_line)
            .unwrap_or_else(|| video_display_size(info, available_width))
    } else {
        video_display_size(info, available_width)
    };
    let bar_height = if resize_enabled {
        VIDEO_RESIZE_BAR_HEIGHT
    } else {
        0.0
    };
    let total_size = Vec2::new(video_size.x, video_size.y + bar_height);
    let (total_rect, _response) = ui.allocate_exact_size(total_size, Sense::hover());

    let video_rect = Rect::from_min_max(
        total_rect.min,
        Pos2::new(total_rect.max.x, total_rect.min.y + video_size.y),
    );
    let resize_bar_rect = if resize_enabled {
        Some(Rect::from_min_max(
            video_rect.left_bottom(),
            total_rect.right_bottom(),
        ))
    } else {
        None
    };

    // Thumbnail/text underlay: visible when the native WebView is hidden (modal occlusion),
    // still loading, or when the WebView path is inactive. The HWND paints above egui when
    // `set_visible(true)`; without this underlay, occlusion left an empty hole in the layout.
    ui.scope_builder(egui::UiBuilder::new().max_rect(video_rect), |ui| {
        render_video_embed_fallback(ui, info, colors, font_size);
    });

    // Hide WebView while dragging so live resize feedback is visible on the thumbnail.
    let source_line = resize.map_or(0, |c| c.source_line);
    let skip_webview = resize
        .filter(|c| c.enabled)
        .is_some_and(|ctx| embed_resize_drag_active(ui, info, ctx.source_line));
    if skip_webview {
        let _ = with_render_slot(|manager, _, key_prefix, _, _| {
            let key = embed_webview_key(key_prefix, info, source_line);
            manager.suppress_embed_for_resize(&key);
        });
    } else {
        let _ = try_render_webview_overlay(ui, info, video_rect, source_line);
    }

    if let (Some(ctx), Some(bar_rect)) = (resize.filter(|c| c.enabled), resize_bar_rect) {
        return handle_video_embed_resize(ui, info, video_rect, bar_rect, ctx.source_line);
    }
    None
}

fn try_render_webview_overlay(
    ui: &mut Ui,
    info: &VideoEmbedInfo,
    rect: Rect,
    source_line: usize,
) -> bool {
    with_render_slot(
        |manager, parent, key_prefix, pane_clip_rect, pixels_per_point| {
            let bounds_rect = match embed_rect_for_webview(ui, rect, pane_clip_rect) {
                Some(r) => r,
                None => return false,
            };
            let visible_rect = embed_visible_portion(ui, bounds_rect, pane_clip_rect);
            let key = embed_webview_key(key_prefix, info, source_line);
            manager.sync_trusted_embed(
                parent,
                &key,
                info,
                bounds_rect,
                visible_rect,
                ui.layer_id(),
                ui.ctx(),
                pixels_per_point,
            )
        },
    )
    .unwrap_or(false)
}

/// Render a video embed using the non-WebView thumbnail/text fallback path.
fn render_video_embed_fallback(
    ui: &mut Ui,
    info: &VideoEmbedInfo,
    colors: &VideoRenderColors,
    font_size: f32,
) {
    let Some(thumbnail_url) = youtube_thumbnail_url(info) else {
        render_text_fallback(ui, info, colors, font_size, None);
        return;
    };

    let cache_id = Id::new("video_embed_thumbnail").with(&thumbnail_url);
    let cached: Option<VideoThumbnailCacheEntry> = ui.data(|d| d.get_temp(cache_id));

    match cached {
        None => {
            // Fetch on a background thread — a blocking HTTP call here froze the
            // UI for up to the request timeout on first render of each thumbnail.
            ui.data_mut(|d| d.insert_temp(cache_id, VideoThumbnailCacheEntry::Loading));
            spawn_thumbnail_fetch(ui.ctx().clone(), thumbnail_url, cache_id);
            render_thumbnail_placeholder(ui, info);
        }
        Some(VideoThumbnailCacheEntry::Loading) => {
            render_thumbnail_placeholder(ui, info);
        }
        Some(VideoThumbnailCacheEntry::Loaded(cached)) => {
            render_thumbnail_widget(ui, info, colors, font_size, &cached);
        }
        Some(VideoThumbnailCacheEntry::Failed) => {
            render_text_fallback(ui, info, colors, font_size, Some("failed"));
        }
    }
}

fn spawn_thumbnail_fetch(ctx: egui::Context, url: String, cache_id: Id) {
    let thread_ctx = ctx.clone();
    let spawned = std::thread::Builder::new()
        .name("video-thumbnail".to_string())
        .spawn(move || {
            let result = match fetch_thumbnail_texture(&thread_ctx, &url) {
                Ok(tex) => VideoThumbnailCacheEntry::Loaded(tex),
                Err(()) => VideoThumbnailCacheEntry::Failed,
            };
            thread_ctx.data_mut(|d| d.insert_temp(cache_id, result));
            thread_ctx.request_repaint();
        });
    if let Err(e) = spawned {
        warn!("Failed to spawn video thumbnail fetch thread: {}", e);
        ctx.data_mut(|d| d.insert_temp(cache_id, VideoThumbnailCacheEntry::Failed));
    }
}

/// Dark placeholder with a play affordance while the thumbnail downloads.
fn render_thumbnail_placeholder(ui: &mut Ui, info: &VideoEmbedInfo) {
    let slot_size = ui.max_rect().size();
    let (rect, response) = ui.allocate_exact_size(slot_size, Sense::click());
    ui.painter()
        .rect_filled(rect, 4.0, Color32::from_black_alpha(200));
    draw_play_overlay(ui, rect);
    handle_video_click(ui, &response, &info.url);
    let _ = response.on_hover_cursor(CursorIcon::PointingHand);
}

fn fetch_thumbnail_texture(ctx: &egui::Context, url: &str) -> Result<CachedVideoThumbnail, ()> {
    let response = ureq::get(url)
        .timeout(Duration::from_secs(10))
        .call()
        .map_err(|e| {
            warn!("Failed to fetch video thumbnail '{}': {}", url, e);
        })?;

    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| {
            warn!("Failed to read video thumbnail '{}': {}", url, e);
        })?;

    let img = image::load_from_memory(&bytes).map_err(|e| {
        warn!("Failed to decode video thumbnail '{}': {}", url, e);
    })?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let pixels: Vec<Color32> = rgba
        .pixels()
        .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();

    let color_image = ColorImage {
        size: [width as usize, height as usize],
        source_size: egui::vec2(width as f32, height as f32),
        pixels,
    };

    let texture_name = format!("md_video_thumb_{}", url);
    let texture = ctx.load_texture(&texture_name, color_image, TextureOptions::LINEAR);

    Ok(CachedVideoThumbnail {
        texture,
        width,
        height,
    })
}

fn render_thumbnail_widget(
    ui: &mut Ui,
    info: &VideoEmbedInfo,
    _colors: &VideoRenderColors,
    _font_size: f32,
    cached: &CachedVideoThumbnail,
) {
    // Match the fixed 16:9 slot from `allocate_exact_size` — never shrink with scroll clip.
    let slot = ui.max_rect();
    let display_w = slot.width().max(1.0);
    let display_h = slot.height().max(1.0);

    let sized = egui::load::SizedTexture::new(cached.texture.id(), Vec2::new(display_w, display_h));
    let image_response = ui.add(
        egui::Image::from_texture(sized)
            .fit_to_exact_size(Vec2::new(display_w, display_h))
            .sense(Sense::click())
            .corner_radius(4.0),
    );

    draw_play_overlay(ui, image_response.rect);
    handle_video_click(ui, &image_response, &info.url);

    let tooltip = if info.trusted {
        t!("markdown.video_embed.play_tooltip").to_string()
    } else {
        t!("markdown.video_embed.untrusted_hint").to_string()
    };
    image_response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(tooltip);
}

fn draw_play_overlay(ui: &Ui, rect: Rect) {
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, Color32::from_black_alpha(60));

    let size = rect.width().min(rect.height()) * 0.18;
    let center = rect.center();
    let circle_rect = Rect::from_center_size(center, Vec2::splat(size));
    painter.circle_filled(
        circle_rect.center(),
        size * 0.5,
        Color32::from_white_alpha(220),
    );

    let tri_w = size * 0.28;
    let tri_h = size * 0.34;
    let offset = tri_w * 0.15;
    let p1 = center + Vec2::new(-tri_w * 0.35 + offset, -tri_h * 0.5);
    let p2 = center + Vec2::new(-tri_w * 0.35 + offset, tri_h * 0.5);
    let p3 = center + Vec2::new(tri_w * 0.65 + offset, 0.0);
    painter.add(Shape::convex_polygon(
        vec![p1, p2, p3],
        Color32::BLACK,
        Stroke::NONE,
    ));
}

fn handle_video_click(ui: &mut Ui, response: &Response, url: &str) {
    if response.clicked() {
        if let Err(e) = open::that(url) {
            error!("Failed to open video URL '{}': {}", url, e);
        }
        ui.memory_mut(|mem| {
            mem.data
                .insert_temp(Id::new("link_click_consumed_this_frame"), true);
        });
    }
}

fn render_text_fallback(
    ui: &mut Ui,
    info: &VideoEmbedInfo,
    colors: &VideoRenderColors,
    font_size: f32,
    mode: Option<&str>,
) {
    let hint = match mode {
        Some("failed") => t!("markdown.video_embed.thumbnail_failed").to_string(),
        None if !info.trusted => t!("markdown.video_embed.untrusted_hint").to_string(),
        _ => t!("markdown.video_embed.open_in_browser").to_string(),
    };

    egui::Frame::new()
        .fill(colors.frame_bg)
        .stroke(Stroke::new(1.0, colors.frame_border))
        .corner_radius(4)
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(hint).color(colors.text).size(font_size));

                let link_label = if info.url.is_empty() {
                    t!("markdown.video_embed.open_in_browser").to_string()
                } else {
                    info.url.clone()
                };

                let link = ui.add(
                    egui::Label::new(RichText::new(link_label).color(colors.link).underline())
                        .sense(Sense::click()),
                );
                if link.on_hover_cursor(CursorIcon::PointingHand).clicked() && !info.url.is_empty()
                {
                    if let Err(e) = open::that(&info.url) {
                        error!("Failed to open video URL '{}': {}", info.url, e);
                    }
                    ui.memory_mut(|mem| {
                        mem.data
                            .insert_temp(Id::new("link_click_consumed_this_frame"), true);
                    });
                }
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parser::VideoProvider;

    fn sample_info(
        provider: VideoProvider,
        video_id: Option<&str>,
        trusted: bool,
    ) -> VideoEmbedInfo {
        VideoEmbedInfo {
            provider,
            video_id: video_id.map(str::to_string),
            url: "https://youtube.com/watch?v=abc123XYZ_-".to_string(),
            trusted,
            width: None,
            height: None,
            source_text: "{{video https://youtube.com/watch?v=abc123XYZ_-}}".to_string(),
        }
    }

    #[test]
    fn youtube_thumbnail_url_from_video_id() {
        let info = sample_info(VideoProvider::YouTube, Some("abc123XYZ_-"), true);
        assert_eq!(
            youtube_thumbnail_url(&info).as_deref(),
            Some("https://img.youtube.com/vi/abc123XYZ_-/hqdefault.jpg")
        );
    }

    #[test]
    fn youtube_thumbnail_url_missing_id() {
        let info = sample_info(VideoProvider::YouTube, None, true);
        assert!(youtube_thumbnail_url(&info).is_none());
    }

    #[test]
    fn youtube_thumbnail_url_empty_id() {
        let info = sample_info(VideoProvider::YouTube, Some(""), true);
        assert!(youtube_thumbnail_url(&info).is_none());
    }

    #[test]
    fn non_youtube_provider_has_no_thumbnail_url() {
        let info = sample_info(VideoProvider::Unknown, None, false);
        assert!(youtube_thumbnail_url(&info).is_none());
    }

    #[test]
    fn trusted_youtube_has_provider_embed_url() {
        let info = sample_info(VideoProvider::YouTube, Some("abc123XYZ_-"), true);
        assert_eq!(
            provider_embed_url(&info).as_deref(),
            Some("ferrite-video://localhost/embed?v=abc123XYZ_-")
        );
    }

    #[test]
    fn relay_html_includes_video_id() {
        let html = youtube_embed_relay_html("abc123XYZ_-");
        assert!(html.contains("abc123XYZ_-"));
        assert!(html.contains("youtube-nocookie.com/embed/"));
        assert!(html.contains("referrerpolicy=\"strict-origin-when-cross-origin\""));
    }

    #[test]
    fn video_id_from_relay_uri_parses_query() {
        assert_eq!(
            video_id_from_relay_uri("ferrite-video://localhost/embed?v=abc123XYZ_-").as_deref(),
            Some("abc123XYZ_-")
        );
    }

    #[test]
    fn untrusted_embed_has_no_provider_embed_url() {
        let info = sample_info(VideoProvider::YouTube, Some("abc123XYZ_-"), false);
        assert!(provider_embed_url(&info).is_none());
    }

    #[test]
    fn untrusted_embed_never_webview_eligible() {
        let info = sample_info(VideoProvider::YouTube, Some("abc123XYZ_-"), false);
        assert!(!VideoWebViewManager::is_webview_eligible(&info));
    }

    #[test]
    fn trusted_youtube_is_webview_eligible() {
        let info = sample_info(VideoProvider::YouTube, Some("abc123XYZ_-"), true);
        assert!(VideoWebViewManager::is_webview_eligible(&info));
    }

    #[test]
    fn webview_gate_blocks_untrusted_before_constructor() {
        let info = sample_info(VideoProvider::YouTube, Some("abc123XYZ_-"), false);
        assert!(
            !VideoWebViewManager::is_webview_eligible(&info),
            "untrusted embed must not reach WebView constructor"
        );
        assert!(provider_embed_url(&info).is_none());
    }

    #[test]
    fn force_fallback_skips_webview_path() {
        let mut manager = VideoWebViewManager::new();
        let info = sample_info(VideoProvider::YouTube, Some("abc123XYZ_-"), true);
        assert!(manager.would_use_webview(&info));
        manager.set_force_fallback(true);
        assert!(!manager.would_use_webview(&info));
    }

    #[test]
    fn clear_all_empties_manager_state() {
        let mut manager = VideoWebViewManager::new();
        manager.seen_this_frame.insert("tab1:abc".to_string());
        manager.failed_embeds.insert("tab1:abc".to_string());
        manager.embed_screen_rects_this_frame.insert(
            "tab1:abc".to_string(),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0)),
        );
        manager.clear_all();
        assert!(manager.webviews.is_empty());
        assert!(manager.seen_this_frame.is_empty());
        assert!(manager.failed_embeds.is_empty());
        assert!(manager.embed_screen_rects_this_frame.is_empty());
    }

    /// Regression: `apply_foreground_occlusion` runs after `end_frame` (once dialogs
    /// and overlays have rendered). If `end_frame` cleared the embed rect map, the
    /// intersection test could never fire and every WebView was re-shown each frame.
    #[test]
    fn embed_rects_survive_end_frame_for_late_occlusion_pass() {
        let mut manager = VideoWebViewManager::new();
        let embed_rect = Rect::from_min_max(Pos2::new(400.0, 100.0), Pos2::new(800.0, 400.0));
        manager.seen_this_frame.insert("tab1:abc".to_string());
        manager
            .embed_screen_rects_this_frame
            .insert("tab1:abc".to_string(), embed_rect);

        let ctx = egui::Context::default();
        manager.end_frame(&ctx);

        assert_eq!(
            manager.embed_screen_rects_this_frame.get("tab1:abc"),
            Some(&embed_rect),
            "embed rects must remain available for apply_foreground_occlusion"
        );

        // Overlapping overlay (e.g. find panel over the video) → obscured.
        let overlay = Rect::from_min_max(Pos2::new(700.0, 50.0), Pos2::new(900.0, 200.0));
        manager.apply_foreground_occlusion(&[overlay]);
        assert!(manager.embed_obscured("tab1:abc"));

        // Non-overlapping overlay (e.g. quick switcher beside the video) → visible.
        let far_overlay = Rect::from_min_max(Pos2::new(0.0, 500.0), Pos2::new(300.0, 700.0));
        manager.apply_foreground_occlusion(&[far_overlay]);
        assert!(!manager.embed_obscured("tab1:abc"));
    }

    /// Regression: a document can embed the same video several times. Keys must be
    /// unique per occurrence or all occurrences share one WebView whose bounds get
    /// re-set once per occurrence per frame (HWND jumps between slots / "doubles").
    #[test]
    fn embed_webview_key_unique_per_source_line() {
        let info = sample_info(VideoProvider::YouTube, Some("abc123XYZ_-"), true);
        let key_a = embed_webview_key("tab1", &info, 12);
        let key_b = embed_webview_key("tab1", &info, 31);
        assert_ne!(
            key_a, key_b,
            "same video on different lines must not collide"
        );
        assert_eq!(
            key_a,
            embed_webview_key("tab1", &info, 12),
            "key must be stable"
        );
        assert_ne!(
            key_a,
            embed_webview_key("tab2", &info, 12),
            "different tabs must not collide"
        );
    }

    #[test]
    fn rects_approx_eq_tolerates_sub_epsilon_jitter() {
        let a = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(420.0, 340.0));
        let jittered = Rect::from_min_max(
            Pos2::new(100.0 + 0.05, 100.0 - 0.05),
            Pos2::new(420.0 - 0.05, 340.0 + 0.05),
        );
        let moved = Rect::from_min_max(Pos2::new(100.0, 130.0), Pos2::new(420.0, 370.0));
        assert!(rects_approx_eq(a, jittered));
        assert!(!rects_approx_eq(a, moved));
    }

    /// Focus is yielded on the pointer-leaves-embeds edge, not every frame —
    /// per-frame `focus_parent` calls disrupted in-flight scrolling.
    #[test]
    fn end_frame_focus_yield_is_edge_triggered() {
        let mut manager = VideoWebViewManager::new();
        let ctx = egui::Context::default();
        assert!(!manager.focus_yielded);
        // No pointer position → yield state becomes active.
        manager.end_frame(&ctx);
        assert!(manager.focus_yielded);
        // Still active on the next frame (no re-trigger; flag stays set).
        manager.end_frame(&ctx);
        assert!(manager.focus_yielded);
    }

    #[test]
    fn begin_frame_resets_embed_rects() {
        let mut manager = VideoWebViewManager::new();
        manager.embed_screen_rects_this_frame.insert(
            "tab1:abc".to_string(),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0)),
        );
        manager.embed_visible_rects_this_frame.insert(
            "tab1:abc".to_string(),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0)),
        );
        manager.begin_frame(Vec::new());
        assert!(manager.embed_screen_rects_this_frame.is_empty());
        assert!(manager.embed_visible_rects_this_frame.is_empty());
    }

    #[test]
    fn container_region_none_when_fully_visible() {
        let embed = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(420.0, 340.0));
        assert_eq!(compute_container_region(embed, embed, 1.0), None);
        // Sub-epsilon differences also count as fully visible.
        let jittered = Rect::from_min_max(Pos2::new(100.2, 100.0), Pos2::new(419.8, 340.0));
        assert_eq!(compute_container_region(embed, jittered, 1.0), None);
    }

    #[test]
    fn container_region_clips_scrolled_top_in_physical_px() {
        let embed = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(420.0, 340.0));
        // Top 40 logical px scrolled out of the pane.
        let visible = Rect::from_min_max(Pos2::new(100.0, 140.0), Pos2::new(420.0, 340.0));
        let region = compute_container_region(embed, visible, 2.0).expect("partial → region");
        assert_eq!(region, (0, 80, 640, 480));
    }

    #[test]
    fn embed_should_show_requires_min_visible_portion() {
        let mut manager = VideoWebViewManager::new();
        let key = "tab1:abc:12";
        let embed = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(420.0, 340.0));
        manager
            .embed_screen_rects_this_frame
            .insert(key.to_string(), embed);

        // Sliver below the minimum → hidden.
        let sliver = Rect::from_min_max(Pos2::new(100.0, 339.5), Pos2::new(420.0, 340.0));
        manager
            .embed_visible_rects_this_frame
            .insert(key.to_string(), sliver);
        assert!(!manager.embed_should_show(key));

        // Half visible → shown when region clipping is available, hidden otherwise.
        let half = Rect::from_min_max(Pos2::new(100.0, 220.0), Pos2::new(420.0, 340.0));
        manager
            .embed_visible_rects_this_frame
            .insert(key.to_string(), half);
        assert_eq!(
            manager.embed_should_show(key),
            super::super::video_webview_input::region_clipping_supported()
        );

        // Fully visible → always shown.
        manager
            .embed_visible_rects_this_frame
            .insert(key.to_string(), embed);
        assert!(manager.embed_should_show(key));
    }

    #[test]
    fn fullscreen_event_for_unknown_embed_is_ignored() {
        let mut manager = VideoWebViewManager::new();
        push_fullscreen_event("tab1:abc:12".to_string(), true);
        manager.begin_frame(Vec::new());
        assert!(
            manager.fullscreen_embeds.is_empty(),
            "fullscreen state must not be tracked for embeds without a WebView"
        );
    }

    #[test]
    fn navigation_allowlist_permits_relay_and_player_only() {
        // Relay page (raw scheme + Windows http/https localhost mappings).
        assert!(is_allowed_webview_navigation(
            "ferrite-video://localhost/embed?v=abc"
        ));
        assert!(is_allowed_webview_navigation(
            "http://ferrite-video.localhost/embed?v=abc"
        ));
        assert!(is_allowed_webview_navigation(
            "https://ferrite-video.localhost/embed?v=abc"
        ));
        // Embed player iframe (WKWebView routes subframe navigations here too).
        assert!(is_allowed_webview_navigation(
            "https://www.youtube-nocookie.com/embed/abc?rel=0"
        ));
        assert!(is_allowed_webview_navigation("about:blank"));

        // Everything else is blocked as top-level document.
        assert!(!is_allowed_webview_navigation(
            "https://www.youtube.com/watch?v=abc"
        ));
        assert!(!is_allowed_webview_navigation("https://evil.example.com/"));
        assert!(!is_allowed_webview_navigation(
            "https://www.youtube-nocookie.com.evil.example.com/"
        ));
        assert!(!is_allowed_webview_navigation(
            "file:///C:/Windows/system32"
        ));
    }

    #[test]
    fn occluder_intersection_respects_margin() {
        let embed = Rect::from_min_max(Pos2::new(100.0, 100.0), Pos2::new(500.0, 400.0));
        let touching = Rect::from_min_max(Pos2::new(500.5, 100.0), Pos2::new(600.0, 200.0));
        let far = Rect::from_min_max(Pos2::new(800.0, 100.0), Pos2::new(900.0, 200.0));
        assert!(embed_rect_intersects_occluders(embed, &[touching]));
        assert!(!embed_rect_intersects_occluders(embed, &[far]));
        assert!(!embed_rect_intersects_occluders(embed, &[]));
    }

    #[test]
    fn yield_focus_when_pointer_in_priority_rect() {
        let embed = Rect::from_min_max(Pos2::new(400.0, 100.0), Pos2::new(800.0, 400.0));
        let raw_pane = Rect::from_min_max(Pos2::ZERO, Pos2::new(380.0, 600.0));
        assert!(should_yield_focus_to_ferrite(
            Some(Pos2::new(50.0, 200.0)),
            &[embed],
            &[raw_pane],
        ));
    }

    #[test]
    fn keep_webview_focus_when_pointer_over_embed() {
        let embed = Rect::from_min_max(Pos2::new(400.0, 100.0), Pos2::new(800.0, 400.0));
        let raw_pane = Rect::from_min_max(Pos2::ZERO, Pos2::new(380.0, 600.0));
        assert!(!should_yield_focus_to_ferrite(
            Some(Pos2::new(500.0, 200.0)),
            &[embed],
            &[raw_pane],
        ));
    }

    #[test]
    fn yield_focus_when_pointer_outside_embed_and_priority() {
        let embed = Rect::from_min_max(Pos2::new(400.0, 100.0), Pos2::new(800.0, 400.0));
        assert!(should_yield_focus_to_ferrite(
            Some(Pos2::new(500.0, 500.0)),
            &[embed],
            &[],
        ));
    }

    #[test]
    fn embed_fully_visible_requires_matching_bounds() {
        let embed = Rect::from_min_max(Pos2::new(100.0, 50.0), Pos2::new(500.0, 275.0));
        assert!(embed_rect_fully_visible_in(embed, embed));
        let partial = Rect::from_min_max(Pos2::new(100.0, 50.0), Pos2::new(500.0, 150.0));
        assert!(!embed_rect_fully_visible_in(partial, embed));
        let shifted = Rect::from_min_max(Pos2::new(100.0, 80.0), Pos2::new(500.0, 305.0));
        assert!(!embed_rect_fully_visible_in(shifted, embed));
    }

    #[test]
    fn video_display_size_default_uses_full_available_width() {
        let info = sample_info(VideoProvider::YouTube, Some("abc"), true);
        let size = video_display_size(&info, 800.0);
        assert_eq!(size.x, 800.0);
        assert_eq!(size.y, 800.0 * EMBED_ASPECT_RATIO);
    }

    #[test]
    fn video_display_size_width_only_uses_16_9() {
        let mut info = sample_info(VideoProvider::YouTube, Some("abc"), true);
        info.width = Some(640);
        let size = video_display_size(&info, 800.0);
        assert_eq!(size.x, 640.0);
        assert_eq!(size.y, 640.0 * EMBED_ASPECT_RATIO);
    }

    #[test]
    fn video_display_size_explicit_width_height() {
        let mut info = sample_info(VideoProvider::YouTube, Some("abc"), true);
        info.width = Some(640);
        info.height = Some(360);
        let size = video_display_size(&info, 800.0);
        assert_eq!(size.x, 640.0);
        assert_eq!(size.y, 360.0);
    }

    #[test]
    fn video_display_size_clamps_to_available_width() {
        let mut info = sample_info(VideoProvider::YouTube, Some("abc"), true);
        info.width = Some(640);
        info.height = Some(360);
        let size = video_display_size(&info, 320.0);
        assert_eq!(size.x, 320.0);
        assert_eq!(size.y, 180.0);
    }

    #[test]
    fn clamp_display_size_scales_down_wide_rect() {
        let size = clamp_display_size(Vec2::new(800.0, 450.0), 400.0);
        assert_eq!(size.x, 400.0);
        assert_eq!(size.y, 225.0);
    }

    /// Regression: drag-resize must accumulate per-frame pointer deltas. Previously the
    /// size was computed as `start + drag_delta` each frame (per-frame delta only), so the
    /// embed never grew during the drag and snapped back to its original size on release.
    #[test]
    fn accumulate_resize_sums_per_frame_deltas() {
        let base = VideoEmbedPendingSize {
            width: 320.0,
            height: 180.0,
        };
        // Three drag frames of +50px each in both axes should total +150px.
        let mut pending = base;
        for _ in 0..3 {
            pending = accumulate_resize(pending, Vec2::new(50.0, 50.0));
        }
        assert_eq!(pending.width, 470.0);
        assert_eq!(pending.height, 330.0);
    }

    #[test]
    fn accumulate_resize_clamps_to_minimum() {
        let base = VideoEmbedPendingSize {
            width: 200.0,
            height: 120.0,
        };
        let pending = accumulate_resize(base, Vec2::new(-500.0, -500.0));
        assert_eq!(pending.width, MIN_EMBED_RESIZE_WIDTH);
        assert_eq!(pending.height, MIN_EMBED_RESIZE_HEIGHT);
    }
}
