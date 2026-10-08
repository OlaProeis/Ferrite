//! macOS Finder "Open With" / double-click path reception (#154).
//!
//! On macOS, opening a document from Finder does **not** put the path in `argv`
//! the way Windows and Linux do — AppKit delivers a `kAEOpenDocuments` Apple
//! Event (forwarded to `application:openURLs:`). Ferrite runs on eframe 0.34 /
//! winit **0.30**, which owns `NSApplicationDelegate` and does **not** expose
//! winit `Event::Opened` or a public app-delegate hook.
//!
//! Upstream: registering a custom delegate before the event loop crashes winit
//! 0.30. Proper user delegates need winit 0.31+ (see
//! <https://github.com/rust-windowing/winit/issues/1751>,
//! <https://github.com/rust-windowing/winit/pull/3758>).
//!
//! ## Best-effort approach (this module)
//!
//! No Objective-C class of our own and no extra crates — C FFI into
//! CoreServices, CoreFoundation, and libobjc:
//!
//! 1. **Cold launch** — observe `NSApplicationWillFinishLaunchingNotification`
//!    and `class_addMethod` an `application:openURLs:` IMP onto winit's
//!    existing delegate class so AppKit's own open-documents handler delivers
//!    URLs (and replies success — no "does not support this file type" dialog).
//! 2. **Warm open** — after eframe creation, install a Carbon
//!    `kAEOpenDocuments` handler that replaces AppKit's dispatch entry.
//!
//! Paths land in a process-wide queue. Call [`get_open_file_paths`] /
//! [`take_opened_files`] to drain. Call [`bind_egui_context`] once from the
//! eframe creator so warm opens can wake the UI.
//!
//! ## Integration notes
//!
//! - [`init_app_delegate`] only registers a CF notification observer — safe
//!   before the event loop (must not crash startup).
//! - Early `main` drains of [`get_open_file_paths`] are usually empty: the
//!   cold-launch event arrives during finish-launching. Drain again from the
//!   eframe `CreationContext` (after launch) for cold-start paths.
//! - Warm-open tab routing: `FerriteApp::handle_macos_open_paths` drains the queue
//!   each frame from the update loop.
//!
//! Approach adapted from FastTIFF's `macos_open` (winit 0.30 + eframe).

use std::ffi::{c_void, CStr, OsString};
use std::os::raw::{c_char, c_long};
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// --- Apple Event Manager FFI (CoreServices.framework) --------------------

/// `AEDesc` / `AppleEvent` / `AEDescList` are all the same opaque descriptor
/// struct in the Apple Event Manager.
#[repr(C)]
struct AEDesc {
    descriptor_type: u32,
    data_handle: *mut c_void,
}

impl AEDesc {
    const fn null() -> Self {
        AEDesc {
            descriptor_type: 0,
            data_handle: std::ptr::null_mut(),
        }
    }
}

/// `AEEventHandlerUPP`: on 64-bit macOS a UPP is just the function pointer.
type AEEventHandlerProc =
    extern "C" fn(event: *const AEDesc, reply: *mut AEDesc, refcon: *mut c_void) -> i16;

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn AEInstallEventHandler(
        event_class: u32,
        event_id: u32,
        handler: AEEventHandlerProc,
        refcon: *mut c_void,
        is_sys_handler: u8,
    ) -> i16;

    fn AEGetParamDesc(
        apple_event: *const AEDesc,
        keyword: u32,
        desired_type: u32,
        result: *mut AEDesc,
    ) -> i16;

    fn AECountItems(list: *const AEDesc, count: *mut c_long) -> i16;

    #[allow(clippy::too_many_arguments)]
    fn AEGetNthPtr(
        list: *const AEDesc,
        index: c_long,
        desired_type: u32,
        keyword: *mut u32,
        type_code: *mut u32,
        data_ptr: *mut c_void,
        maximum_size: c_long,
        actual_size: *mut c_long,
    ) -> i16;

    fn AEDisposeDesc(desc: *mut AEDesc) -> i16;
}

// --- CoreFoundation notification FFI --------------------------------------

type CFNotificationCallback = extern "C" fn(
    center: *mut c_void,
    observer: *mut c_void,
    name: *const c_void,
    object: *const c_void,
    user_info: *const c_void,
);

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFNotificationCenterGetLocalCenter() -> *mut c_void;
    fn CFNotificationCenterAddObserver(
        center: *mut c_void,
        observer: *const c_void,
        callback: CFNotificationCallback,
        name: *const c_void,
        object: *const c_void,
        suspension_behavior: isize,
    );
    fn CFStringCreateWithCString(
        alloc: *const c_void,
        c_str: *const c_char,
        encoding: u32,
    ) -> *const c_void;
}

const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;
const CF_NOTIFICATION_DELIVER_IMMEDIATELY: isize = 4;

// --- Objective-C runtime FFI (libobjc) -------------------------------------

#[link(name = "objc")]
extern "C" {
    /// Untyped; cast per call site to the exact signature before invoking.
    fn objc_msgSend();
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn object_getClass(obj: *mut c_void) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *const c_void;
    fn class_addMethod(
        cls: *mut c_void,
        name: *const c_void,
        imp: *mut c_void,
        types: *const c_char,
    ) -> u8;
}

/// `[obj sel]` returning an object pointer.
///
/// # Safety
/// `obj` must be a valid Objective-C object (or class) and `sel` a selector it
/// responds to with a `()`-args, object-return signature.
unsafe fn msg_obj(obj: *mut c_void, sel: *const c_void) -> *mut c_void {
    let f: unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    f(obj, sel)
}

/// Pack four ASCII bytes into a big-endian `FourCharCode`.
const fn fourcc(code: &[u8; 4]) -> u32 {
    ((code[0] as u32) << 24) | ((code[1] as u32) << 16) | ((code[2] as u32) << 8) | (code[3] as u32)
}

// --- state ---------------------------------------------------------------

/// Paths delivered by Finder "Open With" / double-click, waiting to be drained.
fn queue() -> &'static Mutex<Vec<PathBuf>> {
    static Q: OnceLock<Mutex<Vec<PathBuf>>> = OnceLock::new();
    Q.get_or_init(|| Mutex::new(Vec::new()))
}

/// egui context stashed so a warm-open handler can wake an idle event loop.
fn ctx_slot() -> &'static OnceLock<egui::Context> {
    static C: OnceLock<egui::Context> = OnceLock::new();
    &C
}

/// Queue opened paths and optionally request a repaint.
fn deliver(paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    log::info!(
        "macOS Open With / Apple Event paths received ({}): {:?}",
        paths.len(),
        paths
    );
    if let Ok(mut q) = queue().lock() {
        q.extend(paths);
    }
    if let Some(ctx) = ctx_slot().get() {
        ctx.request_repaint();
    }
}

/// Drain any paths delivered since the last call.
///
/// Prefer calling this from the eframe `CreationContext` (cold launch) and from
/// the update loop via `handle_macos_open_paths` (warm open). An early `main`
/// drain is usually empty because the Apple Event has not been dispatched yet.
pub fn take_opened_files() -> Vec<PathBuf> {
    match queue().lock() {
        Ok(mut q) => std::mem::take(&mut *q),
        Err(_) => Vec::new(),
    }
}

/// Retrieves file paths queued from macOS "Open With" / Apple Events.
///
/// Alias of [`take_opened_files`] for the existing platform API.
pub fn get_open_file_paths() -> Vec<PathBuf> {
    take_opened_files()
}

/// Remember the egui context and install the Carbon open-documents handler.
///
/// Call once from the eframe creator (after AppKit finish-launching). Idempotent
/// for the AE handler; the context slot is set only once.
pub fn bind_egui_context(ctx: egui::Context) {
    let _ = ctx_slot().set(ctx);
    install_ae_handler();
}

/// Launch-time setup: register the `willFinishLaunching` observer that injects
/// `application:openURLs:` onto winit's app delegate.
///
/// Safe to call before the event loop. Does **not** set an NSApplicationDelegate
/// (that crashes winit 0.30 startup).
pub fn init_app_delegate() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| unsafe {
        let name = CFStringCreateWithCString(
            std::ptr::null(),
            c"NSApplicationWillFinishLaunchingNotification".as_ptr(),
            K_CF_STRING_ENCODING_UTF8,
        );
        if name.is_null() {
            log::error!("macOS: couldn't create launch-notification name");
            return;
        }
        // Observer NULL = non-removable; the notification fires once per process.
        CFNotificationCenterAddObserver(
            CFNotificationCenterGetLocalCenter(),
            std::ptr::null(),
            on_will_finish_launching,
            name,
            std::ptr::null(),
            CF_NOTIFICATION_DELIVER_IMMEDIATELY,
        );
        log::debug!("macOS: registered WillFinishLaunching observer for Open With");
    });
}

/// Fires after winit created `NSApplication` and set its delegate, but before
/// AppKit dispatches the queued open-documents event.
extern "C" fn on_will_finish_launching(
    _center: *mut c_void,
    _observer: *mut c_void,
    _name: *const c_void,
    _object: *const c_void,
    _user_info: *const c_void,
) {
    // SAFETY: main thread; NSApplication exists for this notification.
    unsafe { inject_open_urls_method() };
}

/// Add `application:openURLs:` to winit's application-delegate class.
///
/// # Safety
/// Must run on the main thread with `NSApplication` initialized.
unsafe fn inject_open_urls_method() {
    let app_cls = objc_getClass(c"NSApplication".as_ptr());
    if app_cls.is_null() {
        log::error!("macOS: NSApplication class not found");
        return;
    }
    let app = msg_obj(app_cls, sel_registerName(c"sharedApplication".as_ptr()));
    let delegate = if app.is_null() {
        std::ptr::null_mut()
    } else {
        msg_obj(app, sel_registerName(c"delegate".as_ptr()))
    };
    if delegate.is_null() {
        log::error!(
            "macOS: no app delegate at willFinishLaunching; cold-launch Open With won't work \
             (escape hatch: CLI `ferrite /path/to/file.md`; see #154)"
        );
        return;
    }
    let cls = object_getClass(delegate);
    let added = class_addMethod(
        cls,
        sel_registerName(c"application:openURLs:".as_ptr()),
        handle_open_urls as *mut c_void,
        c"v@:@@".as_ptr(), // void (id self, SEL _cmd, id app, id urls)
    );
    if added == 0 {
        log::warn!("macOS: app delegate already implements application:openURLs:");
    } else {
        log::debug!("macOS: injected application:openURLs: onto winit app delegate");
    }
}

/// Injected `application:openURLs:` — `urls` is an `NSArray<NSURL *>`.
extern "C" fn handle_open_urls(
    _this: *mut c_void,
    _cmd: *const c_void,
    _app: *mut c_void,
    urls: *mut c_void,
) {
    // SAFETY: AppKit passes a valid NSArray for the duration of this call.
    deliver(unsafe { ns_urls_to_paths(urls) });
}

/// Read an `NSArray<NSURL *>` into filesystem paths via `fileSystemRepresentation`.
///
/// # Safety
/// `urls` must be a valid `NSArray<NSURL *>` (or null, which yields empty).
unsafe fn ns_urls_to_paths(urls: *mut c_void) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if urls.is_null() {
        return out;
    }
    let count: unsafe extern "C" fn(*mut c_void, *const c_void) -> usize =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let at_index: unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> *mut c_void =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    let fs_repr: unsafe extern "C" fn(*mut c_void, *const c_void) -> *const c_char =
        std::mem::transmute(objc_msgSend as unsafe extern "C" fn());

    let n = count(urls, sel_registerName(c"count".as_ptr()));
    for i in 0..n {
        let url = at_index(urls, sel_registerName(c"objectAtIndex:".as_ptr()), i);
        if url.is_null() {
            continue;
        }
        let repr = fs_repr(url, sel_registerName(c"fileSystemRepresentation".as_ptr()));
        if repr.is_null() {
            continue;
        }
        let bytes = CStr::from_ptr(repr).to_bytes().to_vec();
        out.push(PathBuf::from(OsString::from_vec(bytes)));
    }
    out
}

// --- Carbon Apple Event handler (warm opens) -------------------------------

/// Install the `kAEOpenDocuments` handler. Idempotent.
fn install_ae_handler() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // SAFETY: plain AEM registration; handler is 'static extern "C".
        let err = unsafe {
            AEInstallEventHandler(
                fourcc(b"aevt"),
                fourcc(b"odoc"),
                handle_open_documents,
                std::ptr::null_mut(),
                0,
            )
        };
        if err != 0 {
            log::error!("macOS: failed to install open-file AE handler (error {err})");
        } else {
            log::debug!("macOS: installed kAEOpenDocuments handler for warm Open With");
        }
    });
}

/// C callback: main thread when Finder asks us to open documents.
extern "C" fn handle_open_documents(
    event: *const AEDesc,
    _reply: *mut AEDesc,
    _refcon: *mut c_void,
) -> i16 {
    // SAFETY: `event` is a valid AppleEvent for the lifetime of this call.
    deliver(unsafe { extract_paths(event) });
    0 // noErr
}

/// Pull the file list out of a `kAEOpenDocuments` event as filesystem paths.
///
/// # Safety
/// `event` must be a valid AppleEvent pointer (or null → empty).
unsafe fn extract_paths(event: *const AEDesc) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if event.is_null() {
        return out;
    }

    // keyDirectObject '----', coerced to a descriptor list 'list'.
    let mut list = AEDesc::null();
    if AEGetParamDesc(event, fourcc(b"----"), fourcc(b"list"), &mut list) != 0 {
        return out;
    }

    let mut count: c_long = 0;
    if AECountItems(&list, &mut count) == 0 {
        for i in 1..=count {
            // Each item as a file URL ('furl'): UTF-8 bytes of a file:// URL.
            let mut buf = [0u8; 4096];
            let mut keyword: u32 = 0;
            let mut type_code: u32 = 0;
            let mut actual: c_long = 0;
            let err = AEGetNthPtr(
                &list,
                i,
                fourcc(b"furl"),
                &mut keyword,
                &mut type_code,
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as c_long,
                &mut actual,
            );
            if err == 0 && actual > 0 && (actual as usize) <= buf.len() {
                if let Some(path) = file_url_to_path(&buf[..actual as usize]) {
                    out.push(path);
                }
            }
        }
    }

    AEDisposeDesc(&mut list);
    out
}

/// Convert UTF-8 bytes of a `file://` URL to a filesystem path.
fn file_url_to_path(bytes: &[u8]) -> Option<PathBuf> {
    let s = std::str::from_utf8(bytes).ok()?;
    // Strip scheme + authority: path starts at the first '/' after "file://"
    // ("file:///p" -> "/p"; "file://localhost/p" -> "/p").
    let rest = s.strip_prefix("file://")?;
    let path = &rest[rest.find('/')?..];
    Some(PathBuf::from(OsString::from_vec(percent_decode(
        path.as_bytes(),
    ))))
}

/// Decode `%XX` escapes in a URL path. Malformed `%` escapes pass through.
fn percent_decode(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_url_plain_path() {
        assert_eq!(
            file_url_to_path(b"file:///Users/me/notes.md"),
            Some(PathBuf::from("/Users/me/notes.md"))
        );
    }

    #[test]
    fn file_url_percent_encoded_spaces_and_unicode() {
        let url = b"file:///Users/me/My%20Notes%20%C3%A9.md";
        assert_eq!(
            file_url_to_path(url),
            Some(PathBuf::from("/Users/me/My Notes é.md"))
        );
    }

    #[test]
    fn file_url_localhost_authority_stripped() {
        assert_eq!(
            file_url_to_path(b"file://localhost/tmp/a.md"),
            Some(PathBuf::from("/tmp/a.md"))
        );
    }

    #[test]
    fn file_url_non_file_rejected() {
        assert_eq!(file_url_to_path(b"http://example.com/a.md"), None);
    }

    #[test]
    fn percent_decode_malformed_escape_is_literal() {
        assert_eq!(percent_decode(b"a%2z%"), b"a%2z%");
    }

    #[test]
    fn take_opened_files_drains_queue() {
        // Isolate from any concurrent delivery in the same process.
        let _ = take_opened_files();
        deliver(vec![PathBuf::from("/tmp/a.md"), PathBuf::from("/tmp/b.md")]);
        let got = take_opened_files();
        assert_eq!(
            got,
            vec![PathBuf::from("/tmp/a.md"), PathBuf::from("/tmp/b.md")]
        );
        assert!(take_opened_files().is_empty());
    }
}
