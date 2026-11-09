# Ferrite v0.3.1 — GrokBot re-run gate (round 2 close-out)

Hand this file to GrokBot on **Linux X11** (same environment as `grokbot/New/qa-report-v0.3.1-linux.md`: Xvfb + xfwm4 + Mesa llvmpipe, `run-ferrite.sh`, debug logs under `logs/`).

| Field | Value |
|-------|--------|
| **Branch** | `0.3.1-experimental` |
| **Build to test** | `0.3.1-experimental` HEAD, **`221b6ae` or later** (round-2 close-out commit; includes round 1 `5325e9b` / `97347d3` / `51f5a3a`, direct fixes `5eec844`, PRD `e68f833`, and round-2 PRD tasks 1–33). Note `cargo test --locked` on this commit: 2239 passed (Windows); report the Linux count. |
| **Prior QA baseline** | `8e6ef508aef141b243b961e278861d76f8bc0d48` — full report in `grokbot/New/qa-report-v0.3.1-linux.md` |
| **Checklist** | `docs/technical/platform/v0.3.1-minimal-release-test.md` (Tier 1 gate) |
| **Tier A re-run detail** | `docs/technical/platform/grokbot-qa-tier-a-rerun.md` |

## Pass criteria

1. **Every row in the tables below** is **PASS** or **BLOCKED-by-input/environment** (document why). Any **FAIL** is filed as a **new issue** — do not reopen the round-2 PRD.
2. **Tier 1** (rows 1–22 incl. 4b, 16b, 18b): all **PASS** or **BLOCKED-by-input/environment** only.
3. Attach new screenshots and `logs/ferrite-debug-*.log` on failure.

---

## Round 1 blockers (re-verify scripts)

| ISS | Fix commit | QA rows / script | Expected at re-run |
|-----|------------|------------------|-------------------|
| ISS-1 | `5325e9b` | T1-2, T1-7, T1-17, T1-18, T1-18b; ARR-1/2/4, CPY-3, IND-9, UX-9; T2-35 (ISS-31) | Rendered/Split navigation and click-away commit never hang; app stays responsive; unsaved work savable |
| ISS-2 | `97347d3` | T2-36, VID-1, EXTRA-YT-2/4/6/7, VID-10 | Trusted YouTube embed in Rendered/Split: thumbnail or degrade — **no** `GDK has not been initialized` abort (exit 134) |
| ISS-5 | `97347d3` | T1-14, IMG-X1, IMG-X2, IMG-4, IMG-7 | **Ctrl+V** with PNG on clipboard inserts `![](assets/…)` and writes under `assets/` (X11 primary path) |
| ISS-7 | `51f5a3a` | T1-21, MMD-12 | Type `` ```mermaid `` char-by-char: fence not corrupted by auto-close; no `diamrem` reversal |
| ISS-9 | `51f5a3a` | EXTRA-SET-9a, EXTRA-SET-9b | Invalid JSON / bad field: tolerant load, user-visible toast, corrupt rename — **no** silent full config wipe on exit |

Supporting round-1 rows (same build; re-run if time permits):

| ISS | Fix commit | Rows | Expected |
|-----|------------|------|----------|
| ISS-3 | `5325e9b` | T1-4b, RLD-7 (UTF-16 half) | UTF-16 LE + BOM opens as text in Ferrite |
| ISS-4 | `5325e9b` | T1-15, SIF-1/2/3 | Search-in-Files click lands caret on match |
| ISS-6 | `51f5a3a` | T1-19, UND-1 | One Ctrl+Z after Split preview code-block edit |
| ISS-8 | `5325e9b` | T1-22, MMD-1 (popup) | Mermaid expand / popup path usable (hang class = ISS-1) |

---

## Direct fixes (`5eec844`)

| ISS | QA rows | Expected |
|-----|---------|----------|
| ISS-35 | T3-41 (supp `task_50_table_inline_formatting.md`) | `[**bold link**](url)` renders bold link text, not literal `**` |
| ISS-37 | TTL-6b, EXTRA-SET-1 (Ctrl+,) | With **Use system title bar** ON: Settings gear on ribbon **and/or** Ctrl+, opens Settings |
| ISS-38 | SES-1, SES-5, T3-46 (observation) | Exit → Save: dialog explains Untitled / Quick note; behaviour documented — not silent data loss |
| ISS-43 | EXTRA-SET-5 | Terminal **Auto-load Layout** OFF: opening panel does **not** load `.ferrite/terminal-layout.json` |
| ISS-45 | PRI-3 | Middle-click on tab closes it (Linux, middle-click paste ON) |
| ISS-46 | IMG-X1, IMG-X2, T1-14 | Failed image paste shows toast (even when clipboard read fails — ISS-5 may still fail separately) |
| ISS-53 | UX-11 | Ribbon **MD** quick reference lists Reload (Ctrl+Shift+R) and word wrap (Alt+Z) |
| ISS-55 | RUN-1 | Stop / timeout after loop or sleep: **stdout from before Stop** visible (PYTHONUNBUFFERED) |
| ISS-57 | MMD-3 | `linkStyle 0,1,2 interpolate basis` curves edges (not identical to no linkStyle) |
| ISS-58 | MMD-6 | Unknown `@pos` node id: **warning + diagram still renders** (not full source fallback) |
| ISS-62 | TBL-3, T2-39 | Alt+Z toggles word wrap **without** inserting literal `z` (real keyboard) |
| ISS-63 | SES-6, SES-1, SES-3 | Window/app unsaved dialogs name affected file(s) where applicable |

---

## Round 2 PRD (cyclopsctl tasks 1–33 on `ferrite-v0-3-1-grokbot-qa-round-2-close-out-prd`)

| ISS | Task(s) | QA rows | Expected |
|-----|---------|---------|----------|
| ISS-36 | 1–4 | SES-6 sequence | After session restore, typing at End **appends** — never replaces whole buffer; watch logs for shrink diagnostic |
| ISS-41 | 5–7 | TBL-2, EOL-3/5, T3-41 | Rendered table/paragraph edit in `Tab::content` before save; dirty `*` on first keystroke; Raw pane not stale |
| ISS-40 | 8–9 | UND-3 | Rendered code-block edit: no dropped chars; **one** Ctrl+Z for whole session |
| ISS-39 | 12 | CPY-2 | Mouse selection copy from highlighted code block byte-identical to source |
| ISS-52 | 10–11 | UX-8 | Click `tail` on mixed-format line → caret on `tail` |
| ISS-42 | 13 | UX-9 | Ctrl/middle-click link in list item opens target; plain click still edits |
| ISS-33 | 14 | T3-44, HTML-6 | Adjacent single-line `<div align>` blocks show **their** text, not first document line |
| ISS-44 | 19 | SIF-3 | Case-insensitive `istanbul` matches `İstanbul` |
| ISS-47 | 20 | SPL-2, SPL-5 | `mistaek` flagged; link text `ok` not flagged |
| ISS-48 | 21 | SPL-7 | Squiggles under misspelled words with word wrap ON |
| ISS-51 | 22 | IND-2 | Shift+Tab outdents line with leading hard tab |
| ISS-61 | 23 | TBL-3 | Raw table guides align with body pipes (wrap OFF); wrap ON still no guides (by design) |
| ISS-56 | 24 | EXTRA-SET-7 | Language dropdown lists all shipped locales; ja/zh_Hans keyboard labels translated |
| ISS-30 | 25 | T2-32, EMO-1/2 | **Documented:** monochrome emoji / some blanks — not a colour regression FAIL unless checklist demands colour |
| ISS-34 | 26 | SI-1, T2-37 | Forwarded open raises **owning** window (xfwm4 may still block — note BLOCKED if WM only) |
| ISS-54 | 27 | TTL-6a | Custom title bar: double-click restore size; drag-from-maximized restores (real display preferred) |
| ISS-49 | 18 | STAT-3 | **Closed by design:** Raw Enter does not continue list markers |
| ISS-50 | 18 | EOL-9 | **Closed by design:** mixed EOL preserved on save |
| ISS-32 | 15 | T3-43, MMD-* | **Deferred 0.3.2** — re-run confirms still known; not a gate FAIL unless layout regressed |
| ISS-59 | 17 | MMD-13 | CJK box width within ~1.1× of painted label (or documented ratio if font-limited) |
| ISS-60 | 16 | MMD-10 | Mid-diagram `autonumber off` stops numbering from that point; badges not on message text |

### Settings / title bar (cross-ref)

| Row | ISS | Expected |
|-----|-----|----------|
| T2-SET / TTL-6b Ctrl+, + gear | ISS-37 | See `5eec844` table above |
| EXTRA-SET-5 Auto-load OFF | ISS-43 | See above |
| T2-39 Alt+Z | ISS-62 | See above |

### Observations fixed in tasks 28–32 (no ISS id — spot-check if regressing)

| Observation | Task | Spot-check |
|-------------|------|------------|
| SI-3 | 28 | `--new-instance` does not clear primary crash recovery or overwrite primary `session.json` |
| SI-4 | 29 | Settings toggles persist within ~500 ms / on dialog close |
| GIT-3 | 30 | Save does not immediately trigger spurious external reload |
| CSV-4 | 31 | Large CSV: no “Showing raw content” banner while grid visible |
| SPL-3 | 31 | Spell hint tooltip not drawn over open context menu |
| HTML-4 | 32 | Heading `## … <br> …` and `<kbd>` in headings render (not box / dropped text) |

---

## Suggested run order

1. Tier 1 rows tied to ISS-1, 2, 5, 7 (and 3, 4, 6, 8 if time).
2. `5eec844` table (quick settings / mermaid / paste toasts).
3. Rendered-edit cluster: TBL-2, UND-3, CPY-2, UX-8, UX-9, T3-44.
4. Search / spell / indent / locales: SIF-3, SPL-2/5/7, IND-2, EXTRA-SET-7, TBL-3.
5. SES-6 / ISS-36 hardening script last (session state sensitive).

---

## Related docs

- Round 2 PRD: `docs/ai-workflow/prds/prd-v0.3.1-qa-round2-closeout.md`
- Round 1 PRD: `docs/ai-workflow/prds/prd-v0.3.1-qa-closeout.md`
- Agent brief: `docs/technical/platform/v0.3.1-grok-bot-qa-brief.md`
