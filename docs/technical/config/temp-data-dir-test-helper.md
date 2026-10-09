# Temp data-dir test helper

## Overview

`with_temp_data_dir` (ISS-28 foundation) runs a closure with `FERRITE_DATA_DIR` set to a fresh `tempfile::TempDir` so config and session tests do not write into the real user config directory.

## Behavior

- Defined in `src/config/mod.rs` under `#[cfg(test)]` as `pub(crate)`.
- The env var name is `persistence::DATA_DIR_ENV_VAR` (`FERRITE_DATA_DIR`).
- A process-global mutex serializes callers. The previous value (or absence) is restored when the closure returns, including on panic. The temp directory is removed after that restore.
- The mutex is not reentrant; do not call the helper from inside its own closure.
- While the env var is set, `get_config_dir()` resolves to that temp directory (portable-mode priority 1). That path is the temp dir itself, not `temp/ferrite`.

## Tests that must use it

Wrap any test that calls `get_config_dir()` or a writer that resolves through it (`load_config`, `save_config` / `save_config_silent`, recovery and crash-snapshot helpers, `SnippetManager::new`). Callers today:

- `src/config/persistence.rs` — config dir, config file path, graceful load, silent save
- `src/config/session.rs` — closed-tab recovery blob and snapshot
- `src/state.rs` — `save_recovery_content_excluding` and clean-exit prune
- `src/config/snippets.rs` — trigger lookup via `SnippetManager::new`

`test_save_config_silent_returns_bool` expects `true` and a `config.json` under the temp dir.

## Audit

`config::tests::audit_test_get_config_dir_uses_temp_data_dir` scans `#[cfg(test)]` modules under `src/` and every `.rs` file under `tests/`. A `get_config_dir()` call is a failure unless it is lexically inside a `with_temp_data_dir(...)` argument. The panic lists `path::function` offenders. Production `get_config_dir` definitions are outside those regions and are ignored.
