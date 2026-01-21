# I18n Audit: Hardcoded Strings Found

Generated: 2026-01-20
Updated: 2026-01-20

## Summary
- **Total hardcoded strings found:** 147
- **Strings with existing keys:** 47
- **New keys needed:** ~100
- **Status:** ✅ Most files completed

This audit identifies user-facing strings in the Ferrite codebase that should be internationalized using `t!()` macro.

## Completion Status

| File | Status |
|------|--------|
| src/app.rs | ✅ Completed |
| src/ui/ribbon.rs | ✅ Completed |
| src/ui/file_tree.rs | ✅ Completed |
| src/ui/pipeline.rs | ✅ Completed |
| src/editor/find_replace.rs | ✅ Completed |
| src/markdown/csv_viewer.rs | ✅ Completed |
| src/markdown/tree_viewer.rs | ✅ Completed |
| src/markdown/widgets.rs | ✅ Completed (key UI strings) |

---

## Hardcoded Strings by File

### src/app.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 1020 | `"🔄 Auto-Save Recovery"` | `recovery.auto_save.title` | Yes |
| 1026 | `"An auto-saved backup was found for this file."` | `recovery.auto_save.backup_found` | Yes |
| 1032 | `"Untitled document"` | `recovery.untitled` | Yes |
| 1041 | `"{} seconds ago"` | `time.seconds_ago` | No (new) |
| 1043 | `"{} minutes ago"` | `time.minutes_ago` | No (new) |
| 1045 | `"{} hours ago"` | `time.hours_ago` | No (new) |
| 1047 | `"{} days ago"` | `time.days_ago` | No (new) |
| 1049 | `"Auto-saved: {}"` | `recovery.auto_save.time_label` | No (new) |
| 1053 | `"Would you like to restore the auto-saved content?"` | `recovery.auto_save.restore_question` | Yes |
| 1057 | `"✅ Restore"` | `recovery.auto_save.restore` | Yes |
| 1060 | `"🗑 Discard"` | `recovery.auto_save.discard` | Yes |
| 1072 | `"Restored from auto-save"` | `notification.restored_auto_save` | No (new) |
| 1082 | `"Auto-save discarded"` | `notification.auto_save_discarded` | No (new) |
| 1115 | `"🔄 Recover Previous Session?"` | `recovery.session.title` | Yes |
| 1125 | `"Ferrite detected that your previous session was not closed properly."` | `recovery.session.crash_detected` | Yes |
| 1132 | `"⚠ {} tab(s) had unsaved changes that may be recoverable."` | `recovery.session.tabs_unsaved` | No (new) |
| 1140 | `"Would you like to restore your previous session?"` | `recovery.session.restore_question` | Yes |
| 1146 | `"✓ Restore Session"` | `recovery.session.restore` | Yes |
| 1147 | `"Restore all tabs from the previous session"` | `recovery.session.restore_tooltip` | No (new) |
| 1156 | `"✗ Start Fresh"` | `recovery.session.start_fresh` | Yes |
| 1157 | `"Discard the previous session and start with an empty editor"` | `recovery.session.start_fresh_tooltip` | No (new) |
| 1173 | `"Session restored"` | `notification.session_restored` | No (new) |
| 1335 | `"Close"` | `a11y.close_button` | Yes |
| 1377 | `"Minimize"` | `a11y.minimize_button` | Yes |
| 1411 | `"Exit Fullscreen (F10 or Esc)"` / `"Fullscreen (F10)"` | `tooltip.fullscreen_exit` / `tooltip.fullscreen_enter` | No (new) |
| 1422 | `"Settings (Ctrl+,)"` | `tooltip.settings` | No (new) |
| 1431 | `"Exit Zen Mode (F11)"` | `zen.exit` | Yes (but uses tooltip format) |
| 1433 | `"Enter Zen Mode (F11)"` | `zen.enter` | Yes (but uses tooltip format) |
| 1446-1448 | View mode tooltips (Raw/Split/Rendered) | `tooltip.view_mode.*` | No (new) |
| 1619 | `"Untitled"` | `status.untitled` | No (new) |
| 1621 | `"No file open"` | `status.no_file` | No (new) |
| 1641 | `"Click for recent files & folders\nShift+Click to open in background"` | `tooltip.recent_items` | No (new) |
| 1700 | `"📄 Recent Files"` | `menu.file.recent` | Yes |
| 1738 | `"📁 Recent Folders"` | `workspace.recent_folders` | No (new) |
| 1918 | `"About / Help (F1)"` | `tooltip.about_help` | No (new) |
| 1939 | `"Current Git branch"` | `tooltip.git_branch` | No (new) |
| 1994 | `"Select Delimiter"` | `csv.select_delimiter` | No (new) |
| 1999 | `"⟳ Auto-detect"` | `csv.delimiter_auto` | Yes |
| 2050 | `"Header Row"` | `csv.header_row` | No (new) |
| 2055 | `"⟳ Auto-detect"` | `csv.delimiter_auto` | Yes |
| 2065 | `"✓ First row is header"` | `csv.has_headers_yes` | No (new) |
| 2072 | `"✗ No header row"` | `csv.has_headers_no` | No (new) |
| 2131 | `"File Encoding"` | `encoding.label` | Yes |
| 2626 | `"New tab"` | `tooltip.new_tab` | No (new) |
| 7035 | `"Unsaved Changes"` | `dialog.unsaved_changes.title` | Yes |
| 7060 | `"Save"` | `dialog.unsaved_changes.save` | Yes |
| 7101 | `"Discard"` | `dialog.unsaved_changes.dont_save` | Yes |
| 7113 | `"Cancel"` | `dialog.confirm.cancel` | Yes |
| 7122 | `"Error"` | `common.error` | Yes |
| 7130 | `"OK"` | `common.ok` | Yes |

### src/ui/ribbon.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 238 | `"File"` | `menu.file.label` | Yes |
| 283 | `"💾 Save"` | `menu.file.save` | Yes |
| 290 | `"📥 Save As..."` | `menu.file.save_as` | Yes |
| 307 | `"Edit"` | `menu.edit.label` | Yes |
| 332 | `"Format"` | `menu.format.label` | Yes |
| 419 | `"Heading {}"` | `ribbon.heading_level` | No (new) |
| 532 | `"Format Document (Pretty-print)"` | `ribbon.format_document` | No (new) |
| 542 | `"Validate Syntax"` | `ribbon.validate_syntax` | No (new) |
| 551 | `"Live Pipeline..."` | `ribbon.pipeline` | No (new) |
| 572 | `"Tools"` | `menu.tools.label` | Yes |
| 593 | `"Hide Info Panel"` | `ribbon.hide_info_panel` | No (new) |
| 595 | `"Show Info Panel"` | `ribbon.show_info_panel` | No (new) |
| 598 | `"Toggle Outline"` | `ribbon.toggle_outline` | No (new) |
| 613 | `"Export"` | `menu.file.export` | Yes |
| 619 | `"🌐 Export as HTML"` | `menu.file.export_html` | Yes |
| 626 | `"📋 Copy as HTML"` | `menu.file.export_clipboard` | Yes |
| 627 | `"Copy rendered HTML to clipboard"` | `ribbon.copy_html_tooltip` | No (new) |
| 634 | `"📄 Export as PDF"` | `ribbon.export_pdf` | No (new) |
| 635 | `"Coming soon"` | `ribbon.coming_soon` | No (new) |

### src/ui/file_tree.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 181 | `"Close Workspace"` | `workspace.close_folder` | Yes |
| 549 | `"📄 New File"` | `workspace.new_file` | Yes |
| 553 | `"📁 New Folder"` | `workspace.new_folder` | Yes |
| 560 | `"✏️ Rename"` | `workspace.rename` | Yes |
| 565 | `"🗑️ Delete"` | `workspace.delete` | Yes |
| 572 | `"📂 Reveal in Explorer"` | `tab.reveal_in_explorer` | Yes |
| 580 | `"🔄 Refresh"` | `workspace.refresh` | Yes |
| 479 | `"tracked"` | `git.tracked` | No (new) |
| 480 | `"modified"` | `git.modified` | Yes |
| 481 | `"staged"` | `git.staged` | Yes |
| 482 | `"staged with changes"` | `git.staged_modified` | No (new) |
| 483 | `"untracked"` | `git.untracked` | Yes |
| 484 | `"ignored"` | `git.ignored` | No (new) |
| 485 | `"deleted"` | `git.deleted` | Yes |
| 486 | `"renamed"` | `git.renamed` | Yes |
| 487 | `"conflict"` | `git.conflict` | No (new) |

### src/markdown/widgets.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 213 | `"Decrease level"` | `widgets.heading.decrease_level` | No (new) |
| 221 | `"Increase level"` | `widgets.heading.increase_level` | No (new) |
| 589 | `"Remove item"` | `widgets.list.remove_item` | No (new) |
| 606 | `"+ Add item"` | `widgets.list.add_item` | No (new) |
| 1586 | `"Delete row"` | `widgets.table.delete_row` | Yes |
| 1616 | `"+ Row"` | `widgets.table.add_row` | Yes |
| 1631 | `"Add a new row"` | `widgets.table.add_row_tooltip` | No (new) |
| 1638 | `"+ Column"` | `widgets.table.add_column` | Yes |
| 1653 | `"Add a new column"` | `widgets.table.add_column_tooltip` | No (new) |
| 1665 | `"Delete column:"` | `widgets.table.delete_column_label` | No (new) |
| 1691 | `"Delete column {}"` | `widgets.table.delete_column` | Yes |
| 1706 | `"Align:"` | `widgets.table.align_label` | No (new) |
| 1720 | `"Left aligned"` | `widgets.table.align_left` | Yes |
| 1721 | `"Center aligned"` | `widgets.table.align_center` | Yes |
| 1722 | `"Right aligned"` | `widgets.table.align_right` | Yes |
| 1723 | `"No alignment"` | `widgets.table.align_none` | No (new) |
| 1734 | `"{} (click to cycle)"` | `widgets.table.align_cycle` | No (new) |
| 2292 | `"Copy"` | `widgets.code_block.copy` | Yes |
| 2293 | `"Copy to clipboard"` | `widgets.code_block.copy_tooltip` | Yes |
| 2301 | `"Done"` / `"Edit"` | `widgets.code_block.done` / `widgets.code_block.edit` | Yes |
| 2305 | `"Finish editing"` | `widgets.code_block.finish_tooltip` | No (new) |
| 2307 | `"Edit code"` | `widgets.code_block.edit_tooltip` | No (new) |
| 2598 | `"Edit link"` | `widgets.link.edit` | Yes |
| 2647 | `"Text:"` | `widgets.link.text_label` | No (new) |
| 2667 | `"URL:"` | `widgets.link.url_label` | No (new) |
| 2692 | `"🔗 Open"` | `widgets.link.open` | Yes |
| 2699 | `"Open URL in browser"` | `widgets.link.open_tooltip` | No (new) |
| 2701 | `"Only http/https URLs can be opened"` | `widgets.link.invalid_url` | No (new) |
| 2717 | `"📋 Copy"` | `widgets.link.copy` | Yes |
| 2718 | `"Copy URL to clipboard"` | `widgets.link.copy_tooltip` | No (new) |
| 3191 | `"mermaid"` | `mermaid.badge` | No (new) |
| 3199 | `"▼ Source"` / `"▶ Source"` | `mermaid.hide_source` / `mermaid.show_source` | No (new) |
| 3231 | `"(empty diagram)"` | `mermaid.empty` | No (new) |
| 3377 | `"Render failed: {}"` | `mermaid.rendering_error` | Yes (partial) |

### src/ui/pipeline.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 690 | `"⚡ Live Pipeline"` | `pipeline.title` | Yes |
| 724 | `"(truncated)"` | `pipeline.truncated` | No (new) |
| 735 | `"Close pipeline panel"` | `pipeline.close_tooltip` | No (new) |
| 746 | `"Cancel execution"` | `pipeline.cancel_tooltip` | No (new) |
| 770 | `"jq '.items[]' or yq '.data' ..."` | `pipeline.command_placeholder` | Yes |
| 793 | `"Recent commands"` | `pipeline.recent` | Yes |
| 801 | `"▶ Run"` | `pipeline.run` | Yes |
| 802 | `"Execute command (Enter)"` | `pipeline.run_tooltip` | No (new) |
| 873 | `"stdout"` | `pipeline.stdout` | No (new) |
| 882 | `"(no output)"` | `pipeline.no_output` | Yes |
| 911 | `"stderr"` | `pipeline.stderr` | No (new) |
| 946-951 | Pipeline hint text (multiline) | `pipeline.hint` | No (new) |
| 961 | `"Executing..."` | `pipeline.running` | Yes |
| 970 | `"(command produced no output)"` | `pipeline.no_output_success` | No (new) |

### src/editor/find_replace.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 349 | `"Find and Replace"` | `find.title` | No (new, window title) |
| 388 | `"Find and Replace"` / `"Find"` | `find.title_replace` / `find.title_find` | No (new) |
| 401 | `"Close (Escape)"` | `find.close_tooltip` | No (new) |
| 414 | `"Hide Replace (Ctrl+H)"` | `find.hide_replace` | No (new) |
| 416 | `"Show Replace (Ctrl+H)"` | `find.show_replace` | No (new) |
| 444 | `"Search..."` | `find.placeholder` | Yes |
| 463 | `"No matches"` | `find.no_results` | Yes |
| 485 | `"Replace with..."` | `find.replace_placeholder` | Yes |
| 498 | `"Case Sensitive"` | `find.match_case` | Yes |
| 513 | `"Whole Word"` | `find.whole_word` | Yes |
| 528 | `"Use Regex"` | `find.use_regex` | Yes |
| 549 | `"Previous (Shift+F3)"` | `find.prev_tooltip` | No (new) |
| 561 | `"Next (F3 or Enter)"` | `find.next_tooltip` | No (new) |
| 574 | `"Replace"` | `find.replace` | Yes |
| 576 | `"Replace current match"` | `find.replace_tooltip` | No (new) |
| 585 | `"Replace All"` | `find.replace_all` | Yes |
| 587 | `"Replace all matches"` | `find.replace_all_tooltip` | No (new) |
| 598 | `"Enter/F3: Next • Shift+F3: Prev • Esc: Close"` | `find.keyboard_hints` | No (new) |

### src/markdown/csv_viewer.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 843-844 | `"⚠ Large file ({:.1} MB). Table view may be slow."` | `csv.large_file_warning` | No (new) |
| 847 | `"Dismiss"` | `common.dismiss` | No (new) |
| 850 | `"Show Raw"` | `csv.show_raw` | No (new) |
| 896 | `"⚠ Parse Error:"` | `csv.error` | Yes |

### src/markdown/tree_viewer.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 649 | `"⚠ Large file ({:.1} MB). Tree view may be slow."` | `tree_viewer.large_file_warning` | No (new) |
| 653 | `"Dismiss"` | `common.dismiss` | No (new) |
| 656 | `"Show Raw"` | `tree_viewer.show_raw` | No (new) |
| 669 | `"▼ Expand All"` | `tree_viewer.expand_all` | Yes |
| 672 | `"▶ Collapse All"` | `tree_viewer.collapse_all` | Yes |
| 722 | `"⚠ Parse Error:"` | `tree_viewer.parse_error` | No (new) |
| 988 | `"📋 Copy Path"` | `tree_viewer.copy_path` | Yes |

---

## New Keys to Add to en.yaml

The following keys need to be added to `locales/en.yaml`:

```yaml
# Time relative strings
time:
  seconds_ago: "%{count} seconds ago"
  minutes_ago: "%{count} minutes ago"
  hours_ago: "%{count} hours ago"
  days_ago: "%{count} days ago"

# Recovery notifications
notification:
  restored_auto_save: "Restored from auto-save"
  auto_save_discarded: "Auto-save discarded"
  session_restored: "Session restored"

# Recovery dialog additions
recovery:
  auto_save:
    time_label: "Auto-saved: %{time}"
  session:
    tabs_unsaved: "⚠ %{count} tab(s) had unsaved changes that may be recoverable."
    restore_tooltip: "Restore all tabs from the previous session"
    start_fresh_tooltip: "Discard the previous session and start with an empty editor"

# Tooltips
tooltip:
  fullscreen_exit: "Exit Fullscreen (F10 or Esc)"
  fullscreen_enter: "Fullscreen (F10)"
  settings: "Settings (Ctrl+,)"
  view_mode:
    raw: "Raw mode - Click to switch to Split (%{modifier}+E)"
    split: "Split mode - Click to switch to Rendered (%{modifier}+E)"
    rendered: "Rendered mode - Click to switch to Raw (%{modifier}+E)"
  recent_items: "Click for recent files & folders\nShift+Click to open in background"
  about_help: "About / Help (F1)"
  git_branch: "Current Git branch"
  new_tab: "New tab"

# Status bar
status:
  untitled: "Untitled"
  no_file: "No file open"

# Workspace additions
workspace:
  recent_folders: "📁 Recent Folders"

# CSV additions
csv:
  select_delimiter: "Select Delimiter"
  header_row: "Header Row"
  has_headers_yes: "✓ First row is header"
  has_headers_no: "✗ No header row"
  large_file_warning: "⚠ Large file (%{size} MB). Table view may be slow."
  show_raw: "Show Raw"

# Git additions
git:
  tracked: "tracked"
  staged_modified: "staged with changes"
  ignored: "ignored"
  conflict: "conflict"

# Ribbon additions
ribbon:
  heading_level: "Heading %{level}"
  format_document: "Format Document (Pretty-print)"
  validate_syntax: "Validate Syntax"
  pipeline: "Live Pipeline (%{modifier}+Shift+L)\nPipe document through shell commands"
  hide_info_panel: "Hide Info Panel"
  show_info_panel: "Show Info Panel"
  toggle_outline: "Toggle Outline"
  copy_html_tooltip: "Copy rendered HTML to clipboard"
  export_pdf: "📄 Export as PDF"
  coming_soon: "Coming soon"

# Widget additions
widgets:
  heading:
    decrease_level: "Decrease level"
    increase_level: "Increase level"
  list:
    remove_item: "Remove item"
    add_item: "+ Add item"
  table:
    add_row_tooltip: "Add a new row"
    add_column_tooltip: "Add a new column"
    delete_column_label: "Delete column:"
    align_label: "Align:"
    align_none: "No alignment"
    align_cycle: "%{alignment} (click to cycle)"
  code_block:
    finish_tooltip: "Finish editing"
    edit_tooltip: "Edit code"
  link:
    text_label: "Text:"
    url_label: "URL:"
    open_tooltip: "Open URL in browser"
    invalid_url: "Only http/https URLs can be opened"
    copy_tooltip: "Copy URL to clipboard"

# Mermaid additions
mermaid:
  badge: "mermaid"
  hide_source: "▼ Source"
  show_source: "▶ Source"
  empty: "(empty diagram)"

# Pipeline additions
pipeline:
  truncated: "(truncated)"
  close_tooltip: "Close pipeline panel"
  cancel_tooltip: "Cancel execution"
  run_tooltip: "Execute command (Enter)"
  stdout: "stdout"
  stderr: "stderr"
  hint: "Enter a command above to pipe your document through it.\nExamples:\n• jq '.'           - Format JSON\n• jq '.items[]'    - Extract array items\n• yq '.data'       - Extract YAML field\n• grep 'pattern'   - Search for pattern"
  no_output_success: "(command produced no output)"

# Find additions
find:
  title: "Find and Replace"
  title_replace: "Find and Replace"
  title_find: "Find"
  close_tooltip: "Close (Escape)"
  hide_replace: "Hide Replace (Ctrl+H)"
  show_replace: "Show Replace (Ctrl+H)"
  prev_tooltip: "Previous (Shift+F3)"
  next_tooltip: "Next (F3 or Enter)"
  replace_tooltip: "Replace current match"
  replace_all_tooltip: "Replace all matches"
  keyboard_hints: "Enter/F3: Next • Shift+F3: Prev • Esc: Close"

# Tree viewer additions
tree_viewer:
  large_file_warning: "⚠ Large file (%{size} MB). Tree view may be slow."
  show_raw: "Show Raw"
  parse_error: "⚠ Parse Error:"

# Common additions
common:
  dismiss: "Dismiss"
```

---

## Notes

1. **Icons in translations**: Some strings contain emoji icons (e.g., `"📄 New File"`). These should be kept in the translations as they are visual identifiers.

2. **Keyboard shortcuts in tooltips**: Many tooltips include keyboard shortcuts that vary by platform. Consider using `%{modifier}` placeholder to show Ctrl on Windows/Linux and Cmd on macOS.

3. **Already using t!()**: Several files (like `src/ui/dialogs.rs`, `src/ui/outline_panel.rs`) are already well-internationalized. Focus efforts on `src/app.rs`, `src/ui/ribbon.rs`, and `src/markdown/widgets.rs`.

4. **Technical strings excluded**: File paths, code identifiers, and technical strings were intentionally excluded from this audit.

5. **Format strings**: Strings using `format!()` need to be converted to use the translation's interpolation syntax (e.g., `%{variable}`).

---

## Priority Order for Implementation

1. **High Priority** (user-facing dialogs and common UI):
   - `src/app.rs` - Recovery dialogs, error dialog, unsaved changes dialog
   - `src/editor/find_replace.rs` - Find/Replace panel
   - `src/ui/ribbon.rs` - Main toolbar

2. **Medium Priority** (specialized views):
   - `src/ui/file_tree.rs` - Context menu items
   - `src/ui/pipeline.rs` - Pipeline panel
   - `src/markdown/csv_viewer.rs` and `src/markdown/tree_viewer.rs`

3. **Lower Priority** (widget labels and tooltips):
   - `src/markdown/widgets.rs` - Table, code block, link widgets
