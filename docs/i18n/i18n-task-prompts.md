# I18n Task Prompts for Ferrite

These prompts should be run in separate chats, in order. Each task outputs files that the next task uses as input.

---

## Prompt 1: Audit Hardcoded Strings

Copy everything below the line for Chat 1:

---

### Task: Audit Hardcoded UI Strings in Ferrite

Our Rust/egui app (Ferrite) uses rust-i18n for localization. Locale files are in `locales/`. We need to find ALL hardcoded user-facing strings that should be internationalized.

#### Step 1: Search for hardcoded strings in these patterns

In `src/` directory, search for:
- `ui.button("...")` and `Button::new("...")`
- `ui.label("...")` and `Label::new("...")`
- `selectable_label(_, "...")`
- `.on_hover_text("...")`
- `ui.heading("...")`
- `Window::new("...")`
- `RichText::new("...")`
- `show_error("...")` and `show_toast("...")`
- Menu items and tooltips

Exclude from results:
- Log messages (debug!, info!, warn!, error!)
- Format strings that are already using t!()
- Technical strings (file paths, URLs, code identifiers)
- Emoji-only strings

#### Step 2: For each hardcoded string found, record

- File path and line number
- The hardcoded string
- Suggested locale key (following existing naming: category.subcategory.key)
- Whether a similar key already exists in locales/en.yaml

#### Step 3: Save results to docs/i18n/i18n-audit-hardcoded.md

Use this format:

```
# I18n Audit: Hardcoded Strings Found

Generated: [date]

## Summary
- Total hardcoded strings found: X
- Strings with existing keys: X
- New keys needed: X

## Hardcoded Strings by File

### src/ui/ribbon.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| 290 | "Save As..." | menu.file.save_as | Yes |
| 619 | "Export as HTML" | menu.file.export_html | Yes (different text) |

### src/ui/dialogs.rs

| Line | Hardcoded String | Suggested Key | Existing Key? |
|------|------------------|---------------|---------------|
| ... | ... | ... | ... |

## New Keys to Add to en.yaml

(list any new keys that need to be created)
```

**IMPORTANT: Do NOT make any code changes yet - only audit and document.**

---

## Prompt 2: Fix Hardcoded Strings

Copy everything below the line for Chat 2:

---

### Task: Replace Hardcoded Strings with Translation Keys

Reference the audit file: @docs/i18n/i18n-audit-hardcoded.md

#### Step 1: Read the audit file to understand what needs fixing

#### Step 2: For each hardcoded string in the audit

**If "Existing Key? = Yes":**
- Replace hardcoded string with t!("existing.key")
- Update the locale key value in en.yaml if the text differs

**If "Existing Key? = No":**
- Add the new key to locales/en.yaml
- Add empty key "" to other locale files (de.yaml, ja.yaml, zh_Hans.yaml)
- Replace hardcoded string with t!("new.key")

#### Step 3: Update the audit file

Add a "Status" column to docs/i18n/i18n-audit-hardcoded.md and mark completed items.

#### Translation macro usage

```rust
use rust_i18n::t;

// Simple usage
t!("key.path")

// With parameters
t!("key.path", param = value.to_string())
```

#### Important

- Ensure `use rust_i18n::t;` is imported in each file you modify
- Keep ellipsis (...) in menu items that open dialogs (standard UI convention)
- Run `cargo check` after changes to verify compilation

---

## Prompt 3: Find and Remove Orphaned Keys

Copy everything below the line for Chat 3:

---

### Task: Remove Orphaned Translation Keys

#### Step 1: Extract all USED keys from codebase

Search src/ for all t!("...") patterns and extract the key names.

Save to docs/i18n/i18n-used-keys.txt (one key per line, sorted alphabetically):

```
about.built_with
about.close_hint
about.copyright
dialog.confirm.cancel
...
```

#### Step 2: Extract all DEFINED keys from en.yaml

Parse locales/en.yaml and list all key paths.

Save to docs/i18n/i18n-defined-keys.txt (one key per line, sorted alphabetically):

```
a11y.close_button
a11y.maximize_button
about.built_with
about.close_hint
...
```

#### Step 3: Find orphaned keys

Compare the two files. Keys in defined-keys.txt but NOT in used-keys.txt are ORPHANED.

Save to docs/i18n/i18n-orphaned-keys.md:

```
# Orphaned Translation Keys

Generated: [date]

## Summary
- Total defined keys: X
- Total used keys: X
- Orphaned keys: X

## Orphaned Keys to Remove

| Key | Current Value | Safe to Remove? |
|-----|---------------|-----------------|
| menu.file.label | "File" | Yes - no references |
| menu.file.new | "New" | Verify - check dynamic use |
```

#### Step 4: Remove confirmed orphaned keys

After reviewing the orphaned keys list:
- Remove orphaned keys from ALL locale files (en.yaml, de.yaml, ja.yaml, zh_Hans.yaml)
- Keep the files in sync

#### Important

- Do NOT remove keys until verified they have zero references
- Check for dynamic key patterns like: t!(&format!("prefix.{}", var))
- Some keys might be constructed at runtime - search carefully

---

## Prompt 4: Sync Locale Files

Copy everything below the line for Chat 4:

---

### Task: Synchronize All Locale Files

Reference files:
- @docs/i18n/i18n-used-keys.txt (keys that should exist)
- @locales/en.yaml (source of truth for structure)

#### Step 1: Verify en.yaml has all used keys

Every key in i18n-used-keys.txt must exist in en.yaml. Report any missing keys.

#### Step 2: Sync other locale files to match en.yaml structure

For each file (de.yaml, ja.yaml, zh_Hans.yaml):

1. Add missing keys with empty value ""
2. Remove keys that don't exist in en.yaml
3. Ensure same key ordering as en.yaml

#### Step 3: Generate sync report

Save to docs/i18n/i18n-sync-report.md:

```
# I18n Sync Report

Generated: [date]

## de.yaml
- Keys added: X (list them)
- Keys removed: X (list them)

## ja.yaml
- Keys added: X (list them)
- Keys removed: X (list them)

## zh_Hans.yaml
- Keys added: X (list them)
- Keys removed: X (list them)

## Status: All locale files are now in sync
```

#### Important

- Do NOT overwrite existing translations
- Only add empty strings "" for missing keys
- Empty strings will appear as "needs translation" in Weblate

---

## Prompt 5: Sync Newly Added Language Files (After Git Pull)

Use this prompt after pulling from GitHub if new language files were added by other contributors.

Copy everything below the line for Chat 5:

---

### Task: Sync New Language Files to Match Cleaned Structure

We just pulled changes from GitHub that added new language files (e.g., Norwegian, or others). These new files were created before our i18n cleanup, so they have the old structure with orphaned keys.

Reference files:
- @docs/i18n/i18n-used-keys.txt (the canonical list of keys that should exist)
- @locales/en.yaml (source of truth for structure and key ordering)

#### Step 1: Identify new locale files

Check the locales/ directory for any language files that weren't processed in the previous sync. Compare against what was synced in docs/i18n/i18n-sync-report.md.

New files might include: nb.yaml, no.yaml, nn.yaml (Norwegian variants), or any other new languages.

#### Step 2: For each new language file

1. Read the file and identify its current keys
2. Compare against docs/i18n/i18n-used-keys.txt
3. Remove any keys that are NOT in i18n-used-keys.txt (these are orphaned)
4. Add any keys that ARE in i18n-used-keys.txt but missing from the file (with empty value "")
5. Reorder keys to match the structure in en.yaml

#### Step 3: Preserve existing translations

When removing orphaned keys or restructuring:
- Do NOT delete any translated values for keys that should still exist
- Only remove keys that are confirmed orphaned (not in i18n-used-keys.txt)
- Keep all existing translations intact

#### Step 4: Update the sync report

Append to docs/i18n/i18n-sync-report.md:

```
## New Files Added After Git Pull

### [filename].yaml
- Keys removed (orphaned): X (list them)
- Keys added (missing): X (list them)
- Existing translations preserved: X

## Status: All locale files including new additions are now in sync
```

#### Step 5: Register new language in the app (if needed)

Check if src/config/settings.rs has a Language enum entry for the new language. If not, note it in the report as needing to be added:

```rust
// In src/config/settings.rs, the Language enum needs entries like:
pub enum Language {
    English,
    German,
    Japanese,
    ChineseSimplified,
    Norwegian,  // <-- new language needs to be added here
}
```

Also check the locale_code() and display_name() implementations.

#### Important

- The new files may have translations we want to keep, just with wrong structure
- Be careful not to lose any actual translation work
- After syncing, the new files should have identical key structure to en.yaml

---

## Prompt 6: Final Verification (Run Last)

Use this prompt after completing all other prompts to verify the i18n changes are correct.

Copy everything below the line for Chat 6:

---

### Task: Verify and Test I18n Changes

We've completed a major i18n cleanup. Before pushing, verify everything is correct.

#### Step 1: Verify build compiles

Run `cargo check` to ensure no compilation errors. If there are errors related to missing translation keys, document them.

#### Step 2: Spot-check translation key usage

Pick 5-10 random t!() calls from different files and verify:
1. The key exists in locales/en.yaml
2. The English value makes sense for the context
3. The key path follows naming conventions (category.subcategory.key)

Example files to check:
- src/ui/ribbon.rs
- src/ui/dialogs.rs
- src/ui/settings.rs
- src/app.rs

#### Step 3: Verify locale file consistency

For each locale file, confirm:
1. All files have the same number of keys (use wc -l or similar)
2. Key structure matches en.yaml (spot-check a few sections)
3. No orphaned keys remain (compare against docs/i18n/i18n-used-keys.txt)

Quick check command idea:
```
# Count keys in each file (approximate)
grep -c ": " locales/*.yaml
```

#### Step 4: Test the app briefly

If possible, run `cargo run` and:
1. Switch language in Settings > Appearance > Language
2. Verify UI text changes appropriately
3. Check a few dialogs/panels have translated text (or empty for untranslated)
4. Look for any obvious missing text or broken UI

#### Step 5: Generate verification report

Save to docs/i18n/i18n-verification-report.md:

```
# I18n Verification Report

Generated: [date]

## Build Status
- [ ] cargo check passes
- [ ] No missing key errors

## Locale File Consistency
| File | Key Count | Matches en.yaml? |
|------|-----------|------------------|
| en.yaml | X | (source) |
| de.yaml | X | Yes/No |
| ja.yaml | X | Yes/No |
| zh_Hans.yaml | X | Yes/No |
| nb_NO.yaml | X | Yes/No |
| et.yaml | X | Yes/No |

## Spot Checks
| File | Key | Status |
|------|-----|--------|
| ribbon.rs | menu.file.save_as | OK |
| ... | ... | ... |

## Runtime Test (if performed)
- [ ] Language switching works
- [ ] UI text updates correctly
- [ ] No obvious missing text

## Issues Found
(list any problems discovered)

## Conclusion
[ ] Ready to push
[ ] Needs fixes (see issues above)
```

#### Important

- This is a read-only verification, don't make changes unless critical issues found
- Document any problems for follow-up
- If major issues found, do NOT push until resolved
