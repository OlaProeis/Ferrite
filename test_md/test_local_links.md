# Local Link Open Test Fixture (v0.3.1)

Used by the v0.3.1 manual test checklist (`LNK-*`). Tests PR #157 — clicking a
**local** file/folder link reveals it in the OS file manager; **web** links open
in the browser. Open this file in **Rendered** view and click each link.

> Behaviour: directories open directly in the file manager; files reveal their
> **parent** folder; HTTP(S) opens the browser; `mailto:`/`tel:`/`ftp:` etc. are
> ignored. Targets resolve relative to **this document**, then the workspace root.

## Relative file links (reveal parent folder)

- [Sibling markdown — flowcharts](test_flowcharts.md)
- [Sibling markdown — tables](test_tables.md)
- [CSV data file](test_data.csv)
- [Nested file](encoding_tests/utf8_no_bom.md)

**Expect:** clicking reveals `test_md/` (or `test_md/encoding_tests/`) in the
system file manager with the target selected/parent opened.

## Relative folder links (open directly)

- [encoding_tests folder](encoding_tests/)
- [test subfolder](test/)

**Expect:** the folder opens directly in the file manager.

## Web links (open in browser)

- [Ferrite repo](https://github.com/OlaProeis/Ferrite)
- <https://example.com>

**Expect:** opens the default browser; no file-manager window.

## Non-resolving / ignored targets

- [Missing file](this_file_does_not_exist.md)
- [Email](mailto:nobody@example.com)
- [Anchor only](#local-link-open-test-fixture-v031)

**Expect:** missing file → nothing happens (no crash); `mailto:` → ignored by the
local-path resolver; in-page anchor → scrolls within the document (no file
manager).
