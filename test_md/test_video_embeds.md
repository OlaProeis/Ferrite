# Video Embed Test Fixture (v0.3.1)

Used by the v0.3.1 manual test checklist (`VID-*`). Open in **Rendered** and
**Split** view. Video WebView playback is **primary-window only** — secondary
windows must fall back to the thumbnail + play overlay.

> Syntax: `{{video URL}}` with optional `width` / `height` (px, clamped 1..=8192).
> Bare YouTube links must **not** auto-embed — they stay normal links.

## 1. Trusted YouTube — default size

{{video https://www.youtube.com/watch?v=dQw4w9WgXcQ}}

**Expect:** thumbnail + play overlay; clicking play loads the wry WebView
(relay page → `youtube-nocookie.com` iframe). No *Error 153*. 16:9 sized to pane.

## 2. Trusted YouTube — short URL

{{video https://youtu.be/dQw4w9WgXcQ}}

**Expect:** same as #1 (host allowlisted).

## 3. Explicit width only (16:9 derived)

{{video https://www.youtube.com/watch?v=dQw4w9WgXcQ width=480}}

**Expect:** player width 480 px, height auto ≈ 270 px.

## 4. Explicit width + height

{{video https://www.youtube.com/watch?v=dQw4w9WgXcQ width=320 height=240}}

**Expect:** player is exactly 320×240.

## 5. Untrusted host (no WebView)

{{video https://vimeo.com/76979871}}

**Expect:** **never** creates a WebView; thumbnail/placeholder only, or opens in
system browser on click. Navigation allowlist blocks non-YouTube hosts.

## 6. Bare link must NOT embed

Here is a plain link: https://www.youtube.com/watch?v=dQw4w9WgXcQ — this should
render as a clickable **link**, not a video player.

And an autolink in a list:

- <https://www.youtube.com/watch?v=dQw4w9WgXcQ>

**Expect:** both stay `Link` nodes; no embed.

## 7. Drag-resize (regression — must not snap back)

Resize embed #1 or #3 using the bottom-right handle.

**Expect:** the player grows/shrinks **continuously** while dragging and
**keeps** the new size on release (no snap back to original). Source line is
rewritten with the new `width`/`height`. Below the minimum it clamps, not
collapses.

## 8. Modal occlusion

With a video playing, open the command palette (Alt+Space), Quick Switcher
(Ctrl+P), Find/Replace, and the unsaved-changes dialog.

**Expect:** each overlay paints **above** the player where it overlaps; the
video stays visible in non-overlapping regions and is not destroyed.

## 9. Same video embedded multiple times (regression)

Embeds #1–#4 above all use the **same** video ID. Scroll so that two or more
of them are fully visible at once, then resize the window.

**Expect:** each embed keeps its **own** player at its own position and size.
No flickering between thumbnail and player, no player jumping/"doubling" into
another embed's slot, and scrolling stays smooth while players are visible.

## 10. Scroll with cursor over a video (regression)

Scroll the page so the cursor ends up **inside** a live player, then keep
scrolling. Also scroll an embed half out of the pane.

**Expect:** wheel scrolling keeps working with the cursor over the video.
Partially scrolled embeds keep showing the **live player, clipped** at the pane
edge (no swap to thumbnail while scrolling). Scrolling an embed out and back
within a couple of seconds does **not** restart playback.

## 11. Fullscreen

Play embed #1 and click the player's fullscreen button. Press ESC (or the
button again) to exit.

**Expect:** the player expands to fill the **Ferrite window** (not the OS
screen) and restores to its embed slot on exit. While fullscreen, the wheel
controls the player (the document does not scroll underneath).
