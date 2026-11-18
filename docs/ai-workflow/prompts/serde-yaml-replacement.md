# Replace abandoned `serde_yaml`

Paste this whole file into a new chat. Do this crate only. Do not commit unless asked.

## Why

`serde_yaml` 0.9.34+deprecated is unmaintained (last publish 2024-03-25). Ferrite uses it to parse and write YAML, not only to load config. `serde_yml` is also unmaintained: its crate page says to move to `noyalib`, which is still `0.0.x` (0.0.62, published 2026-11-17). Do not take `noyalib` or `serde_yml`.

Use **`serde_norway` 0.9** (latest 0.9.42). It is the maintained continuation of the `serde_yaml` 0.9 API (`Value`, `Mapping`, `Number`, `Tagged`, `from_str`, `to_string`).

## Repo

- Path: `G:\DEV\markDownNotepad`
- One branch for this change. Do not bump `notify`, `comrak`, `egui`, or `rust-i18n` in the same branch.

## What to change

In `Cargo.toml`, replace `serde_yaml = "0.9"` with `serde_norway = "0.9"`. Rename `serde_yaml::` to `serde_norway::` at every direct use. If a type Ferrite matches on is missing (`Value::Tagged` in particular), stop and report it. Do not switch crates mid-task.

`Mapping` must keep key order. The frontmatter panel walks the map in file order.

Call sites:

| File | Use |
|------|-----|
| `src/markdown/tree_viewer.rs` | `from_str` → tree, and tree → `to_string`. Matches `Null`, `Bool`, `Number` (`as_i64` / `as_f64` / `Number::from`), `String`, `Sequence`, `Mapping`, `Tagged` (unwrap `tagged.value`). |
| `src/ui/frontmatter_panel.rs` | `parse_frontmatter_fields`: mapping order, scalars, sequences, and `Tagged` serialized back to text. |
| `src/markdown/mermaid/frontmatter.rs` | `from_str::<FrontmatterYaml>` for `title` and `config.theme`. A parse error must still return `(None, original source)` so the diagram renders. |
| `src/spellcheck/mod.rs` | Test-only locale walker (`yaml_leaf_string`, `flatten_yaml_keys`). |
| `src/ui/settings/mod.rs` | Test-only locale walker. |

`rust-i18n` 3.1.2 pulls its own `serde_yml` through `rust-i18n-macro` and `rust-i18n-support`. Leave that dependency alone. Replacing Ferrite's direct crate does not remove `serde_yml` from `Cargo.lock`.

## Tests

Existing:

- `cargo test --bin ferrite test_parse_yaml`
- `cargo test --bin ferrite frontmatter`
- `cargo test --bin ferrite locale`

Add one YAML round-trip in the tree viewer tests if none exists: a mapping whose key order is `z` then `a`, plus a nested sequence. Parsing and serializing must keep `z` before `a`.

`cargo clippy --bin ferrite` on the files you touched.

## Manual check

- Open a `.yaml` file in the tree viewer, change a value, and confirm the text view still parses.
- Open a markdown note with frontmatter and confirm the properties card lists the keys in file order.
- Open a mermaid block whose source starts with `---\ntitle: My Chart\n---`. The title must still parse, and a broken frontmatter block must still render the diagram.

## Docs

Update the `serde_yaml` mentions in:

- `docs/technical/viewers/tree-viewer.md`
- `docs/technical/ui/frontmatter-panel.md`
- `docs/technical/mermaid/mermaid-frontmatter.md`

Say the parser is `serde_norway` 0.9, and that `serde_yaml` was abandoned. Leave `docs/refactor.md` alone.

## Leave alone

- `notify`, `comrak`, `egui`, `eframe`, and `rust-i18n`.
- Locale file contents. This task only changes the crate that reads them.
- Do not mark tasks done.
