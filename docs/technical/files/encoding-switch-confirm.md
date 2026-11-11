# Encoding switch confirmation (#197)

The status-bar encoding picker re-decodes `original_bytes` through `Tab::set_encoding`, which assigns that text to both `content` and `original_content`. On a dirty tab that drops unsaved edits and clears the modified flag.

`AppState::request_encoding_change` asks first when the tab is modified and `original_bytes` is non-empty. The prompt is `show_encoding_switch_dialog` (its own flag, not the close-tab / exit confirm dialog):

- **Cancel** keeps the edits and the current encoding.
- **Don't Save** re-decodes from the last saved bytes.
- **Save** writes the buffer with the encoding it already has, then re-decodes the bytes just written. If the save does not happen, the encoding stays.

A clean tab still re-decodes immediately. A new unsaved document has no `original_bytes`, so the picker only changes the label used at save time and does not ask.
