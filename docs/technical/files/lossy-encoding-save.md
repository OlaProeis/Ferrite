# Lossy legacy-encoding save (#198)

Saving a document whose encoding cannot represent every character is refused. `encoding_rs` would otherwise replace those characters with HTML numeric character references (`&#NNNN;`) and report success. Ferrite does not write those bytes.

`Tab::encode_content` returns `Err(UnmappableEncode)` when `encoding.encode` sets `had_errors`. `save_tab_by_id` and `save_active_tab_as` stop before writing and open `show_encoding_lossy_save_dialog`.

- **Save as UTF-8** sets the tab encoding to UTF-8, writes UTF-8 bytes, and marks the tab saved the same way a normal save does.
- **Cancel** leaves the buffer, the encoding, and the file untouched.

UTF-8 and the hand-written UTF-16 paths have no lossy replacement, so they save without this dialog.
