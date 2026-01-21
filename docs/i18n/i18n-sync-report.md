# I18n Sync Report

Generated: 2026-01-20

## Summary

All locale files have been synchronized to match `en.yaml` structure (388 keys).

---

## de.yaml

- **Keys added:** 0
- **Keys removed:** 0

**Status:** Already in sync. File structure matches `en.yaml` exactly.

---

## ja.yaml

- **Keys added:** 0
- **Keys removed:** 0

**Status:** Already in sync. File structure matches `en.yaml` exactly.

---

## zh_Hans.yaml

- **Keys added:** 0 (all keys already existed, but some were in duplicate "additions" sections)
- **Keys removed:** 0 (duplicate section declarations were consolidated)

**Restructuring performed:**
The file had structural issues with duplicate YAML sections appended at the bottom ("additions" sections). These have been consolidated into the proper structure:

1. **Removed duplicate section declarations:**
   - `status` section (was declared twice)
   - `find` section (was declared twice)
   - `workspace` section (was declared twice)
   - `csv` section (was declared twice)
   - `widgets` section (was declared twice)
   - `mermaid` section (was declared twice)
   - `pipeline` section (was declared twice)
   - `tooltip` section (was declared twice)
   - `ribbon` section (was declared twice)

2. **Preserved all existing translations** - No translation values were lost

3. **Keys with empty values "" (need translation):**
   - `status.untitled`
   - `status.no_file`
   - `find.title`
   - `find.title_replace`
   - `find.title_find`
   - `find.close_tooltip`
   - `find.hide_replace`
   - `find.show_replace`
   - `find.prev_tooltip`
   - `find.next_tooltip`
   - `find.replace_tooltip`
   - `find.replace_all_tooltip`
   - `find.keyboard_hints`
   - `notification.restored_auto_save`
   - `notification.auto_save_discarded`
   - `notification.session_restored`
   - `tree_viewer.show_raw`
   - `tree_viewer.parse_error`
   - `git.tracked`
   - `git.staged_modified`
   - `git.ignored`
   - `git.conflict`
   - `recovery.auto_save.title`
   - `recovery.auto_save.backup_found`
   - `recovery.auto_save.restore_question`
   - `recovery.auto_save.restore`
   - `recovery.auto_save.discard`
   - `recovery.session.title`
   - `recovery.session.crash_detected`
   - `recovery.session.restore_question`
   - `recovery.session.restore`
   - `recovery.session.start_fresh`
   - `recovery.session.restore_tooltip`
   - `recovery.session.start_fresh_tooltip`
   - `recovery.untitled`
   - `common.ok`
   - `common.copy`
   - `common.error`
   - `common.dismiss`
   - `widgets.code_block.copy_tooltip`
   - `widgets.code_block.finish_tooltip`
   - `widgets.code_block.edit_tooltip`
   - `widgets.table.add_row`
   - `widgets.table.add_column`
   - `widgets.table.align_left`
   - `widgets.table.align_center`
   - `widgets.table.align_right`
   - `widgets.table.delete_column_label`
   - `widgets.table.align_label`
   - `widgets.table.align_none`
   - `widgets.link.edit`
   - `widgets.link.open`
   - `widgets.link.copy`
   - `widgets.link.text_label`
   - `widgets.link.url_label`
   - `widgets.link.copy_tooltip`
   - `csv.delimiter_auto`
   - `csv.error`
   - `csv.select_delimiter`
   - `csv.header_row`
   - `csv.has_headers_yes`
   - `csv.has_headers_no`
   - `csv.show_raw`
   - `pipeline.title`
   - `pipeline.command_placeholder`
   - `pipeline.run`
   - `pipeline.recent`
   - `pipeline.no_output`
   - `pipeline.running`
   - `pipeline.truncated`
   - `pipeline.close_tooltip`
   - `pipeline.cancel_tooltip`
   - `pipeline.run_tooltip`
   - `pipeline.stdout`
   - `pipeline.stderr`
   - `pipeline.hint`
   - `pipeline.no_output_success`
   - `tab.reveal_in_explorer`
   - `mermaid.badge`
   - `mermaid.empty`
   - `workspace.close_folder`
   - `workspace.new_file`
   - `workspace.new_folder`
   - `workspace.rename`
   - `workspace.delete`
   - `workspace.refresh`
   - `workspace.recent_folders`
   - `zen.enter`
   - `zen.exit`
   - `tooltip.fullscreen_exit`
   - `tooltip.fullscreen_enter`
   - `tooltip.settings`
   - `tooltip.recent_items`
   - `tooltip.about_help`
   - `tooltip.git_branch`
   - `tooltip.new_tab`
   - `ribbon.format_document`
   - `ribbon.validate_syntax`
   - `ribbon.pipeline`
   - `ribbon.hide_info_panel`
   - `ribbon.show_info_panel`
   - `ribbon.toggle_outline`
   - `ribbon.copy_html_tooltip`
   - `ribbon.export_pdf`
   - `ribbon.coming_soon`

**Status:** Restructured successfully. All 388 keys present with proper ordering.

---

---

## New Files Added After Git Pull (2026-01-20)

### et.yaml (Estonian)

- **Keys removed (orphaned):** ~65 keys from old structure
- **Keys added (missing):** 0 (file restructured to canonical 388 keys)
- **Existing translations preserved:** 0 (file had no translations, all empty strings)

**Orphaned keys removed (partial list):**
- `app.tagline`
- `menu.file.new`, `menu.file.close`, `menu.file.close_all`, `menu.file.clear_recent`, `menu.file.quit`
- `menu.edit.*` (undo, redo, cut, copy, paste, select_all, find, find_replace, go_to_line, etc.)
- `menu.view.*`, `menu.help.*` (entire sections)
- `toolbar.*` (entire section)
- `view_mode.raw`, `view_mode.rendered`, `view_mode.split` (labels without _desc)
- `status.line`, `status.column`, `status.words`, `status.characters`, `status.modified`, `status.saved`, `status.encoding`, `status.language`
- `dialog.open.*`, `dialog.save.*`, `dialog.export.*`
- `sidebar.*`, `error.*`, `encoding.*`, `toc.*`, `format_toolbar.*`, `log_level.*`, `drag_drop.*`, `export.*`
- And ~40 more orphaned keys from old structure

**Status:** Restructured successfully. All 388 canonical keys present with proper ordering.

---

### nb_NO.yaml (Norwegian Bokmål)

- **Keys removed (orphaned):** ~65 keys from old structure
- **Keys added (missing):** 0 (file restructured to canonical 388 keys)
- **Existing translations preserved:** 27

**Norwegian translations preserved:**
- `menu.file.label`: "Fil"
- `menu.file.open`: "Åpne…"
- `menu.file.save`: "Lagre"
- `menu.file.save_as`: "Lagre som…"
- `menu.file.recent`: "Nylige filer"
- `menu.file.export`: "Eksporter"
- `menu.file.export_html`: "Eksporter til HTML..."
- `menu.file.export_clipboard`: "Kopier som HTML"
- `menu.edit.label`: "Rediger"
- `menu.format.label`: "Format"
- `menu.tools.label`: "Verktøy"
- `view_mode.raw_desc`: "Ren Markdown-tekstredigering"
- `view_mode.rendered_desc`: "WYSIWYG-gjengitt redigering"
- `view_mode.split_desc`: "Råredigering + gjengitt forhåndsvisning side ved side"
- `dialog.unsaved_changes.title`: "Ulagrede endringer"
- `dialog.unsaved_changes.save`: "Lagre"
- `dialog.unsaved_changes.dont_save`: "Ikke lagre"
- `dialog.confirm.cancel`: "Avbryt"
- `dialog.confirm.close`: "Lukk"
- `dialog.file.new_file`: "Ny fil"
- `dialog.file.new_folder`: "Ny mappe"
- `dialog.file.enter_file_name`: "Skriv inn filnavn:"
- `dialog.file.enter_folder_name`: "Skriv inn mappenavn:"
- `dialog.file.hint_file`: "filnavn.md"
- `dialog.file.hint_folder`: "mappe-navn"
- `dialog.file.create`: "Opprett"
- `dialog.file.rename`: "Gi nytt navn"

**Orphaned keys removed (same categories as et.yaml)**

**Status:** Restructured successfully. All 388 canonical keys present with proper ordering. 27 Norwegian translations preserved.

---

## Language Enum Registration Status

The `Language` enum in `src/config/settings.rs` currently only includes:
- `English` (en)
- `ChineseSimplified` (zh_Hans)

**Languages missing from enum (need to be added):**
- `German` (de) - de.yaml exists
- `Japanese` (ja) - ja.yaml exists
- `Estonian` (et) - et.yaml exists
- `NorwegianBokmal` (nb_NO) - nb_NO.yaml exists

To add these languages, update `src/config/settings.rs`:

```rust
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "zh-Hans")]
    ChineseSimplified,
    #[serde(rename = "et")]
    Estonian,
    #[serde(rename = "nb-NO")]
    NorwegianBokmal,
}
```

And update the `locale_code()`, `native_name()`, and `all()` implementations accordingly.

---

## Status: All locale files including new additions are now in sync

All six locale files (`en.yaml`, `de.yaml`, `ja.yaml`, `zh_Hans.yaml`, `et.yaml`, `nb_NO.yaml`) now have:
- Identical key structure (388 keys)
- Same key ordering
- Valid YAML syntax (no duplicate sections)

Empty string values `""` indicate keys that need translation in Weblate.
