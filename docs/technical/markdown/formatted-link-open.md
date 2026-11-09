# Formatted-block link open (ISS-42)

## Problem

Paragraphs, list items, and table cells that use the formatted LayoutJob path paint links as underlined text. The block-level click-to-edit interact runs on the whole block, so Ctrl/Cmd+click and middle-click on a link never opened the target.

## Solution

After the galley is laid out, `screen_link_hits` (`src/markdown/widgets.rs`) maps each inline link's display-char range onto glyph rects.

`link_click_action(modifiers, button)` classifies the gesture:

| Gesture | Action |
|---------|--------|
| Middle-click, or primary click with Ctrl/Cmd | `Open` |
| Plain primary click | `Edit` |
| Any other button | `None` |

`take_open_link_click` runs before click-to-edit. On `Open` it consumes the click (`link_click_consumed_this_frame`).

- URL / autolink destinations go through `resolve_openable_link_target` and `open_link_target` (browser or file manager).
- `[[wikilink]]` targets are stored as `wikilink_clicked_target` so the app opens a tab.
- Success logs `Opened link target via …` (`modifier-click`, `middle-click`, or `popup`).
- Failure logs `Failed to open link target` and queues a toast. `FerriteApp` drains it with `drain_link_open_toasts`.

Call sites: formatted paragraphs and list items (`formatted_link_pointer` in `editor.rs`, structural and non-structural) and table-cell display mode in `widgets.rs`. Hover over a link uses the pointing-hand cursor.

HTML blocks are out of scope.

## Tests

| Test | Coverage |
|------|----------|
| `link_click_action_opens_on_modifier_or_middle_and_edits_on_plain` | Pure Open / Edit / None |
| `link_click_on_list_item_opens_target_and_plain_click_edits` | List item harness: Ctrl+primary click fires the open callback and stays out of source edit; plain click sets `formatted_editing` |
| `resolve_openable_link_target_accepts_https_and_rejects_empty` | Existing URL resolver |
