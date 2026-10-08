//! Linux X11/Wayland primary selection (middle-click paste).
//!
//! egui has no primary-selection API. On Linux this module talks to the OS
//! clipboard via arboard's `LinuxClipboardKind::Primary`. Other platforms get
//! no-op stubs so the editor can call the same functions unconditionally.

use std::time::{Duration, Instant};

/// Debounce window before a new editor selection is published.
pub const PRIMARY_PUBLISH_DEBOUNCE: Duration = Duration::from_millis(150);

/// Decide whether the current raw-editor selection should be published.
///
/// `armed_at` is the publish deadline from a previous call (`None` if unarmed).
/// Returns `(publish, next_deadline)`:
/// - a new range selection arms `now + 150ms` and does not publish yet
/// - an elapsed deadline with a pointer button still down waits
/// - an elapsed deadline with the pointer up publishes once
/// - an identical already-published range does not re-publish
#[must_use]
pub fn primary_publish_decision(
    selection: Option<(usize, usize)>,
    now: Instant,
    pointer_down: bool,
    last_published: Option<(usize, usize)>,
    armed_at: Option<Instant>,
) -> (bool, Option<Instant>) {
    let Some(sel) = selection else {
        return (false, None);
    };
    if last_published == Some(sel) {
        return (false, None);
    }

    let deadline = armed_at.unwrap_or_else(|| now + PRIMARY_PUBLISH_DEBOUNCE);
    if now < deadline {
        return (false, Some(deadline));
    }
    if pointer_down {
        return (false, Some(deadline));
    }
    (true, None)
}

/// Publish `text` to the Linux primary selection. Empty text is ignored.
pub fn set_primary(text: &str) {
    if text.is_empty() {
        return;
    }
    set_primary_impl(text);
}

/// Read the Linux primary selection. Returns `None` when empty or unavailable.
#[must_use]
pub fn get_primary() -> Option<String> {
    get_primary_impl()
}

#[cfg(target_os = "linux")]
fn set_primary_impl(text: &str) {
    use arboard::{LinuxClipboardKind, SetExtLinux};

    with_clipboard(|clipboard| {
        let _ = clipboard
            .set()
            .clipboard(LinuxClipboardKind::Primary)
            .text(text.to_owned());
    });
}

#[cfg(target_os = "linux")]
fn get_primary_impl() -> Option<String> {
    use arboard::{GetExtLinux, LinuxClipboardKind};

    with_clipboard(|clipboard| {
        clipboard
            .get()
            .clipboard(LinuxClipboardKind::Primary)
            .text()
            .ok()
    })
    .flatten()
    .filter(|text| !text.is_empty())
}

#[cfg(target_os = "linux")]
thread_local! {
    static CLIPBOARD: std::cell::RefCell<Option<arboard::Clipboard>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(target_os = "linux")]
fn with_clipboard<T>(f: impl FnOnce(&mut arboard::Clipboard) -> T) -> Option<T> {
    CLIPBOARD.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = arboard::Clipboard::new().ok();
        }
        slot.as_mut().map(f)
    })
}

#[cfg(not(target_os = "linux"))]
fn set_primary_impl(_text: &str) {}

#[cfg(not(target_os = "linux"))]
fn get_primary_impl() -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_selection_arms_deadline() {
        let now = Instant::now();
        let (publish, next) = primary_publish_decision(Some((0, 4)), now, false, None, None);
        assert!(!publish);
        assert_eq!(next, Some(now + PRIMARY_PUBLISH_DEBOUNCE));
    }

    #[test]
    fn elapsed_with_pointer_down_does_not_publish() {
        let armed = Instant::now();
        let now = armed;
        let (publish, next) = primary_publish_decision(Some((2, 8)), now, true, None, Some(armed));
        assert!(!publish);
        assert_eq!(next, Some(armed));
    }

    #[test]
    fn elapsed_with_pointer_up_publishes_once() {
        let armed = Instant::now();
        let now = armed;
        let (publish, next) = primary_publish_decision(Some((2, 8)), now, false, None, Some(armed));
        assert!(publish);
        assert!(next.is_none());
    }

    #[test]
    fn identical_selection_does_not_republish() {
        let now = Instant::now();
        let sel = Some((1, 6));
        let (publish, next) = primary_publish_decision(sel, now, false, sel, None);
        assert!(!publish);
        assert!(next.is_none());

        let (publish, next) = primary_publish_decision(sel, now, false, sel, Some(now));
        assert!(!publish);
        assert!(next.is_none());
    }

    #[test]
    fn new_selection_after_publish_rearms() {
        let now = Instant::now();
        let (publish, next) =
            primary_publish_decision(Some((4, 10)), now, false, Some((0, 4)), None);
        assert!(!publish);
        assert_eq!(next, Some(now + PRIMARY_PUBLISH_DEBOUNCE));
    }

    #[test]
    fn collapsed_selection_clears_deadline() {
        let now = Instant::now();
        let (publish, next) = primary_publish_decision(None, now, false, Some((0, 3)), Some(now));
        assert!(!publish);
        assert!(next.is_none());
    }

    #[test]
    fn set_primary_skips_empty_and_is_safe() {
        set_primary("");
        set_primary("ferrite-primary");
        let _ = get_primary();
    }
}
