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
use wry::raw_window_handle::{HandleError, HasWindowHandle, RawWindowHandle, WindowHandle};
use rust_i18n::t;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::time::Duration;
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response as HttpResponse};
use wry::{
    NewWindowResponse, Rect as WryRect, WebView, WebViewBuilder, WebViewId,
};

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
    /// Whether each synced embed lies fully inside scroll/pane clips this frame.
    embed_scroll_fully_visible_this_frame: HashMap<String, bool>,
    /// Ferrite UI rects that must receive input instead of the WebView (e.g. split raw pane).
    focus_priority_rects: Vec<Rect>,
    /// Foreground overlay rects from the last occlusion pass (screen space).
    foreground_occluders: Vec<Rect>,
    force_fallback: bool,
}

struct ActiveWebView {
    webview: WebView,
    loaded_url: String,
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
            embed_scroll_fully_visible_this_frame: HashMap::new(),
            focus_priority_rects: Vec::new(),
            foreground_occluders: Vec::new(),
            force_fallback: false,
        }
    }

    /// Mark the start of a rendered-view frame (clears the active-embed set).
    pub fn begin_frame(&mut self, focus_priority_rects: Vec<Rect>) {
        self.seen_this_frame.clear();
        self.embed_screen_rects_this_frame.clear();
        self.embed_scroll_fully_visible_this_frame.clear();
        self.focus_priority_rects = focus_priority_rects;
    }

    /// Screen rects of synced embeds this frame (for wheel-hook hit testing).
    pub fn embed_screen_rects(&self) -> Vec<Rect> {
        self.embed_screen_rects_this_frame.values().copied().collect()
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

        let embed_rects: Vec<Rect> = self.embed_screen_rects_this_frame.values().copied().collect();
        if should_yield_focus_to_ferrite(pointer_pos, &embed_rects, &priority) {
            for entry in self.webviews.values() {
                let _ = entry.webview.focus_parent();
            }
        }

        let stale: Vec<String> = self
            .webviews
            .keys()
            .filter(|key| !self.seen_this_frame.contains(*key))
            .cloned()
            .collect();
        for key in stale {
            if let Some(entry) = self.webviews.remove(&key) {
                let _ = entry.webview.focus_parent();
            }
        }

        // Deliberately keep `embed_screen_rects_this_frame`: the foreground-occlusion
        // pass (`apply_foreground_occlusion`) runs after end_frame, once dialogs and
        // overlays have rendered, and needs these rects for intersection tests.
        // The map is cleared at the start of the next rendered frame in `begin_frame`.
    }

    /// Destroy every active child WebView (tab switch, Raw mode, inactive tab, etc.).
    pub fn clear_all(&mut self) {
        for entry in self.webviews.values() {
            let _ = entry.webview.focus_parent();
        }
        self.webviews.clear();
        self.seen_this_frame.clear();
        self.failed_embeds.clear();
        self.embed_screen_rects_this_frame.clear();
        self.embed_scroll_fully_visible_this_frame.clear();
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
        let scroll_fully_visible = self
            .embed_scroll_fully_visible_this_frame
            .get(embed_key)
            .copied()
            .unwrap_or(false);
        let obscured = self.embed_obscured(embed_key);
        let show = scroll_fully_visible && !obscured;
        let Some(entry) = self.webviews.get_mut(embed_key) else {
            return;
        };
        if !show {
            let _ = entry.webview.focus_parent();
        }
        let _ = entry.webview.set_visible(show);
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
    pub fn sync_trusted_embed(
        &mut self,
        parent: &VideoWebViewParent,
        embed_key: &str,
        info: &VideoEmbedInfo,
        rect: Rect,
        scroll_fully_visible: bool,
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

        let bounds = match egui_rect_to_wry_bounds(ctx, layer_id, rect, pixels_per_point) {
            Some(bounds) => bounds,
            None => return false,
        };

        self.seen_this_frame.insert(embed_key.to_string());
        self.embed_screen_rects_this_frame.insert(
            embed_key.to_string(),
            rect_to_viewport(ctx, layer_id, rect),
        );
        self.embed_scroll_fully_visible_this_frame
            .insert(embed_key.to_string(), scroll_fully_visible);

        let existing_sync = if let Some(entry) = self.webviews.get_mut(embed_key) {
            if entry.loaded_url != url {
                let _ = entry.webview.focus_parent();
                false
            } else if entry.webview.set_bounds(bounds).is_ok() {
                true
            } else {
                let _ = entry.webview.focus_parent();
                false
            }
        } else {
            false
        };

        if self.webviews.get(embed_key).is_some() && !existing_sync {
            self.webviews.remove(embed_key);
            return false;
        }

        if existing_sync {
            self.apply_visibility_for_key(embed_key);
            if let Some(entry) = self.webviews.get(embed_key) {
                super::video_webview_input::install_wheel_forwarding(&entry.webview);
            }
            return true;
        }

        let video_id = info.video_id.as_deref().unwrap_or_default();
        match create_child_webview(parent, video_id, &url, bounds) {
            Ok(webview) => {
                self.webviews.insert(
                    embed_key.to_string(),
                    ActiveWebView {
                        webview,
                        loaded_url: url,
                    },
                );
                self.apply_visibility_for_key(embed_key);
                super::video_webview_input::install_wheel_forwarding(
                    &self.webviews[embed_key].webview,
                );
                true
            }
            Err(()) => {
                self.failed_embeds.insert(embed_key.to_string());
                false
            }
        }
    }
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
    video_id: &str,
    relay_url: &str,
    bounds: WryRect,
) -> Result<WebView, ()> {
    if !is_valid_youtube_video_id(video_id) {
        return Err(());
    }

    WebViewBuilder::new()
        .with_custom_protocol(VIDEO_EMBED_PROTOCOL.to_string(), serve_youtube_embed_relay)
        .with_url(relay_url)
        .with_bounds(bounds)
        .with_focused(false)
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

fn embed_scroll_fully_visible(ui: &Ui, embed_rect: Rect, pane_clip_rect: Rect) -> bool {
    let visible = embed_rect
        .intersect(ui.clip_rect())
        .intersect(pane_clip_rect);
    embed_rect_fully_visible_in(visible, embed_rect)
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

fn egui_rect_to_wry_bounds(
    ctx: &egui::Context,
    layer_id: LayerId,
    rect: Rect,
    _pixels_per_point: f32,
) -> Option<WryRect> {
    let global_rect = rect_to_viewport(ctx, layer_id, rect);

    if !global_rect.is_positive() {
        return None;
    }

    Some(WryRect {
        position: LogicalPosition::new(global_rect.min.x, global_rect.min.y).into(),
        size: LogicalSize::new(global_rect.width(), global_rect.height()).into(),
    })
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

#[derive(Clone)]
struct CachedVideoThumbnail {
    texture: TextureHandle,
    width: u32,
    height: u32,
}

#[derive(Clone)]
enum VideoThumbnailCacheEntry {
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

fn video_display_size(available_width: f32) -> Vec2 {
    let width = available_width.max(1.0);
    Vec2::new(width, width * EMBED_ASPECT_RATIO)
}

/// Render a video embed, preferring the WebView path for trusted embeds when context is set.
pub fn render_video_embed(
    ui: &mut Ui,
    info: &VideoEmbedInfo,
    colors: &VideoRenderColors,
    font_size: f32,
) {
    let display_size = video_display_size(ui.available_width());
    let (rect, _response) = ui.allocate_exact_size(display_size, Sense::hover());

    // Thumbnail/text underlay: visible when the native WebView is hidden (modal occlusion),
    // still loading, or when the WebView path is inactive. The HWND paints above egui when
    // `set_visible(true)`; without this underlay, occlusion left an empty hole in the layout.
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        render_video_embed_fallback(ui, info, colors, font_size);
    });

    let _ = try_render_webview_overlay(ui, info, rect);
}

fn try_render_webview_overlay(ui: &mut Ui, info: &VideoEmbedInfo, rect: Rect) -> bool {
    with_render_slot(|manager, parent, key_prefix, pane_clip_rect, pixels_per_point| {
        let bounds_rect = match embed_rect_for_webview(ui, rect, pane_clip_rect) {
            Some(r) => r,
            None => return false,
        };
        let scroll_fully_visible = embed_scroll_fully_visible(ui, bounds_rect, pane_clip_rect);
        let key = format!("{}:{}", key_prefix, embed_stable_key(info));
        manager.sync_trusted_embed(
            parent,
            &key,
            info,
            bounds_rect,
            scroll_fully_visible,
            ui.layer_id(),
            ui.ctx(),
            pixels_per_point,
        )
    })
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

    let load_result = cached.unwrap_or_else(|| {
        let result = match fetch_thumbnail_texture(ui.ctx(), &thumbnail_url) {
            Ok(tex) => VideoThumbnailCacheEntry::Loaded(tex),
            Err(()) => VideoThumbnailCacheEntry::Failed,
        };
        ui.data_mut(|d| d.insert_temp(cache_id, result.clone()));
        result
    });

    match load_result {
        VideoThumbnailCacheEntry::Loaded(cached) => {
            render_thumbnail_widget(ui, info, colors, font_size, &cached);
        }
        VideoThumbnailCacheEntry::Failed => {
            render_text_fallback(ui, info, colors, font_size, Some("failed"));
        }
    }
}

fn fetch_thumbnail_texture(
    ctx: &egui::Context,
    url: &str,
) -> Result<CachedVideoThumbnail, ()> {
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
    painter.circle_filled(circle_rect.center(), size * 0.5, Color32::from_white_alpha(220));

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
                ui.label(
                    RichText::new(hint)
                        .color(colors.text)
                        .size(font_size),
                );

                let link_label = if info.url.is_empty() {
                    t!("markdown.video_embed.open_in_browser").to_string()
                } else {
                    info.url.clone()
                };

                let link = ui.add(
                    egui::Label::new(
                        RichText::new(link_label)
                            .color(colors.link)
                            .underline(),
                    )
                    .sense(Sense::click()),
                );
                if link
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .clicked()
                    && !info.url.is_empty()
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

    fn sample_info(provider: VideoProvider, video_id: Option<&str>, trusted: bool) -> VideoEmbedInfo {
        VideoEmbedInfo {
            provider,
            video_id: video_id.map(str::to_string),
            url: "https://youtube.com/watch?v=abc123XYZ_-".to_string(),
            trusted,
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

    #[test]
    fn begin_frame_resets_embed_rects() {
        let mut manager = VideoWebViewManager::new();
        manager.embed_screen_rects_this_frame.insert(
            "tab1:abc".to_string(),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(100.0, 100.0)),
        );
        manager
            .embed_scroll_fully_visible_this_frame
            .insert("tab1:abc".to_string(), true);
        manager.begin_frame(Vec::new());
        assert!(manager.embed_screen_rects_this_frame.is_empty());
        assert!(manager.embed_scroll_fully_visible_this_frame.is_empty());
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
        assert!(!is_allowed_webview_navigation("https://www.youtube.com/watch?v=abc"));
        assert!(!is_allowed_webview_navigation("https://evil.example.com/"));
        assert!(!is_allowed_webview_navigation(
            "https://www.youtube-nocookie.com.evil.example.com/"
        ));
        assert!(!is_allowed_webview_navigation("file:///C:/Windows/system32"));
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
}
